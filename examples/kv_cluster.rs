use std::time::Duration;

use raft_service_rs::controller::AddLearnerRequest;
use raft_service_rs::controller::ChangeMembershipRequest;
use raft_service_rs::pb::controller::InitRequest;
use raft_service_rs::pb::controller::raft_controller_service_client::RaftControllerServiceClient;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::service::Application;
use crate::service::node_address;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_target(true)
        .with_thread_ids(true)
        .with_level(true)
        .with_ansi(false)
        .with_line_number(true)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let raft1 = {
        let tmp = TempDir::new()?;
        Application::new(1, node_address(1).rpc_addr, tmp.path().to_path_buf()).await?
    };

    let raft2 = {
        let tmp = TempDir::new()?;
        Application::new(2, node_address(2).rpc_addr, tmp.path().to_path_buf()).await?
    };

    let raft3 = {
        let tmp = TempDir::new()?;
        Application::new(3, node_address(3).rpc_addr, tmp.path().to_path_buf()).await?
    };

    let shutdown = CancellationToken::new();

    let _h1 = tokio::spawn({
        let shutdown = shutdown.clone();
        async move { raft1.run(shutdown).await }
    });
    let _h2 = tokio::spawn({
        let shutdown = shutdown.clone();
        async move { raft2.run(shutdown).await }
    });
    let _h3 = tokio::spawn({
        let shutdown = shutdown.clone();
        async move { raft3.run(shutdown).await }
    });

    // Wait for server to start up.
    tokio::time::sleep(Duration::from_millis(200)).await;

    let mut client1 =
        RaftControllerServiceClient::connect(format!("http://{}", node_address(1).rpc_addr))
            .await?;
    let response = client1
        .init(InitRequest {
            nodes: vec![node_address(1)],
        })
        .await?
        .into_inner();
    info!(?response);
    tokio::time::sleep(Duration::from_millis(500)).await;

    let response = client1
        .add_learner(AddLearnerRequest {
            node: Some(node_address(2)),
        })
        .await?
        .into_inner();
    info!(?response);
    tokio::time::sleep(Duration::from_millis(500)).await;

    let response = client1
        .add_learner(AddLearnerRequest {
            node: Some(node_address(3)),
        })
        .await?
        .into_inner();
    info!(?response);
    tokio::time::sleep(Duration::from_millis(500)).await;

    let response = client1
        .change_membership(ChangeMembershipRequest {
            members: vec![1, 2, 3],
            retain: false,
        })
        .await?
        .into_inner();
    info!(?response);
    tokio::time::sleep(Duration::from_millis(500)).await;

    shutdown.cancelled().await;

    Ok(())
}

mod service {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use prost::DecodeError;
    use raft_service_rs::Node;
    use raft_service_rs::application::ApplicationConfig;
    use raft_service_rs::application::ApplicationData;
    use raft_service_rs::application::ApplicationLayer;
    use raft_service_rs::orchestrator::RaftOrchestrator;
    use raft_service_rs::server::RaftDataClient;
    use raft_service_rs::server::RaftServiceConfig;
    use tokio::task::JoinSet;
    use tokio_util::sync::CancellationToken;
    use tonic::async_trait;

    use crate::service::reading_service::ReadingServiceBuilder;
    use crate::service::worker::Worker;
    use crate::service::writing_service::WritingServiceBuilder;

    pub fn node_address(id: u64) -> Node {
        Node {
            node_id: id,
            rpc_addr: format!("127.0.0.1:{}", 21000 + id),
        }
    }

    mod worker {
        use tokio_util::sync::CancellationToken;
        use tonic::async_trait;

        #[async_trait]
        pub trait Worker {
            async fn run(&self, cancel: CancellationToken);
        }
    }

    mod writing_service {
        use std::time::Duration;

        use raft_service_rs::server::RaftDataClient;
        use tokio::time::sleep;
        use tokio_util::sync::CancellationToken;
        use tonic::async_trait;
        use tracing::info;

        use crate::service::KeyValueServiceConfig;
        use crate::service::Request;
        use crate::service::worker::Worker;

        pub struct WritingService {
            raft_client: RaftDataClient<KeyValueServiceConfig>,
        }

        #[async_trait]
        impl Worker for WritingService {
            async fn run(&self, cancel: CancellationToken) {
                let raft_client = self.raft_client.clone();

                let mut start = 0;

                while !cancel.is_cancelled() {
                    let request = Request {
                        key: start.to_string(),
                        value: start.to_string(),
                    };

                    info!(node_id = raft_client.node_id(), ?request);

                    raft_client.write(&request).await.unwrap();

                    sleep(Duration::from_secs(1)).await;
                    start += 1;
                }
            }
        }

        pub struct WritingServiceBuilder {
            raft_client: RaftDataClient<KeyValueServiceConfig>,
        }

        impl WritingServiceBuilder {
            pub fn new(raft_client: RaftDataClient<KeyValueServiceConfig>) -> Self {
                WritingServiceBuilder { raft_client }
            }

            pub fn build(&self) -> WritingService {
                WritingService {
                    raft_client: self.raft_client.clone(),
                }
            }
        }
    }

    mod reading_service {
        use std::time::Duration;

        use raft_service_rs::server::RaftDataClient;
        use tokio::time::sleep;
        use tokio_util::sync::CancellationToken;
        use tonic::async_trait;
        use tracing::info;

        use crate::service::KeyValueServiceConfig;
        use crate::service::worker::Worker;

        pub struct ReadingService {
            raft_client: RaftDataClient<KeyValueServiceConfig>,
        }

        #[async_trait]
        impl Worker for ReadingService {
            async fn run(&self, cancel: CancellationToken) {
                let raft_client = self.raft_client.clone();

                while !cancel.is_cancelled() {
                    let len = raft_client
                        .read_safe(|store| store.map.len())
                        .await
                        .unwrap();

                    info!(node_id = raft_client.node_id(), len);

                    sleep(Duration::from_secs(1)).await;
                }
            }
        }

        pub struct ReadingServiceBuilder {
            raft_client: RaftDataClient<KeyValueServiceConfig>,
        }

        impl ReadingServiceBuilder {
            pub fn new(
                raft_client: RaftDataClient<KeyValueServiceConfig>,
            ) -> ReadingServiceBuilder {
                ReadingServiceBuilder { raft_client }
            }

            pub fn build(&self) -> ReadingService {
                ReadingService {
                    raft_client: self.raft_client.clone(),
                }
            }
        }
    }

    #[derive(prost::Message)]
    pub struct Request {
        #[prost(string, tag = "1")]
        key: String,
        #[prost(string, tag = "2")]
        value: String,
    }

    #[derive(prost::Message)]
    pub struct Snapshot {
        #[prost(map = "string, string", tag = "1")]
        map: HashMap<String, String>,
    }

    #[derive(Default)]
    pub struct KeyValueData {
        map: HashMap<String, String>,
    }

    #[async_trait]
    impl ApplicationData for KeyValueData {
        type Request = Request;

        type ApplicationSnapshot = Snapshot;

        fn export(&self) -> Self::ApplicationSnapshot {
            Snapshot {
                map: self.map.clone(),
            }
        }

        fn import(snapshot: Self::ApplicationSnapshot) -> Result<Self, DecodeError> {
            Ok(KeyValueData { map: snapshot.map })
        }

        async fn apply(&mut self, request: Self::Request) -> anyhow::Result<bool> {
            self.map.insert(request.key, request.value);

            Ok(true)
        }
    }

    #[derive(Clone, Default)]
    pub struct KeyValueServiceConfig;

    impl ApplicationConfig for KeyValueServiceConfig {
        type Data = KeyValueData;
    }

    struct KeyValueService {
        writing_service_builder: WritingServiceBuilder,
        reading_service_builder: ReadingServiceBuilder,
        join_set: Option<JoinSet<()>>,
    }

    #[async_trait]
    impl ApplicationLayer for KeyValueService {
        type C = KeyValueServiceConfig;
        type Config = ();

        async fn new(
            _config: Self::Config,
            raft_client: RaftDataClient<Self::C>,
            _shutdown: CancellationToken,
        ) -> anyhow::Result<Self> {
            Ok(KeyValueService {
                writing_service_builder: WritingServiceBuilder::new(raft_client.clone()),
                reading_service_builder: ReadingServiceBuilder::new(raft_client),
                join_set: None,
            })
        }

        async fn start(&mut self, cancel: CancellationToken) -> anyhow::Result<()> {
            let mut join_set = JoinSet::new();

            {
                let server = self.reading_service_builder.build();
                let cancel = cancel.clone();
                join_set.spawn(async move {
                    server.run(cancel).await;
                });
            }

            {
                let server = self.writing_service_builder.build();
                join_set.spawn(async move {
                    server.run(cancel).await;
                });
            }

            self.join_set = Some(join_set);

            Ok(())
        }

        async fn wait_to_stop(&mut self) -> anyhow::Result<()> {
            if let Some(join_set) = self.join_set.take() {
                join_set.join_all().await;
            }

            Ok(())
        }

        async fn shutdown(self) -> anyhow::Result<()> {
            Ok(())
        }
    }

    pub struct Application {
        raft_config: RaftServiceConfig,
    }

    impl Application {
        pub async fn new(node_id: u64, rpc_url: String, log_path: PathBuf) -> anyhow::Result<Self> {
            let raft_config = RaftServiceConfig {
                node_id,
                rpc_url,
                log_path,
            };

            Ok(Self { raft_config })
        }

        pub async fn run(self, shutdown: CancellationToken) -> anyhow::Result<()> {
            let orchestrator =
                RaftOrchestrator::<KeyValueService>::new(self.raft_config, (), shutdown).await?;

            orchestrator.run().await?;

            Ok(())
        }
    }
}
