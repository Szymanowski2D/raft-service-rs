use std::sync::Arc;
use std::time::Duration;

use maplit::hashmap;
use raft_service_rs::Node;
use tempfile::TempDir;
use tracing_subscriber::EnvFilter;

use crate::key_value_service::KeyValueService;

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn test_cluster() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_target(true)
        .with_thread_ids(true)
        .with_level(true)
        .with_ansi(false)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let raft1 = {
        let tmp = TempDir::new()?;
        Arc::new(KeyValueService::new(1, 21001, tmp.path()).await?)
    };

    let raft2 = {
        let tmp = TempDir::new()?;
        Arc::new(KeyValueService::new(2, 21002, tmp.path()).await?)
    };

    let raft3 = {
        let tmp = TempDir::new()?;
        Arc::new(KeyValueService::new(3, 21003, tmp.path()).await?)
    };

    let _h1 = tokio::spawn({
        let raft = raft1.clone();
        async move { raft.run().await }
    });

    let _h2 = tokio::spawn({
        let raft = raft2.clone();
        async move { raft.run().await }
    });

    let _h3 = tokio::spawn({
        let raft = raft3.clone();
        async move { raft.run().await }
    });

    // Wait for server to start up.
    tokio::time::sleep(Duration::from_millis(200)).await;

    raft1
        .initialize(hashmap! {
            1 => node_address(1)
        })
        .await?;

    tokio::time::sleep(Duration::from_millis(500)).await;
    raft1.add_learner(node_address(2)).await?;

    tokio::time::sleep(Duration::from_millis(500)).await;
    raft1.add_learner(node_address(3)).await?;

    tokio::time::sleep(Duration::from_millis(500)).await;
    raft1.add_voter(node_address(2)).await?;

    tokio::time::sleep(Duration::from_millis(500)).await;
    raft1.add_voter(node_address(3)).await?;

    tokio::time::sleep(Duration::from_secs(60)).await;

    Ok(())
}

fn node_address(id: u64) -> Node {
    Node {
        node_id: id,
        rpc_addr: format!("127.0.0.1:{}", 21000 + id),
    }
}

mod key_value_service {
    use std::collections::HashMap;
    use std::net::IpAddr;
    use std::net::Ipv4Addr;
    use std::net::SocketAddr;
    use std::path::Path;

    use prost::DecodeError;
    use raft_service_rs::ApplicationConfig;
    use raft_service_rs::ApplicationData;
    use raft_service_rs::Node;
    use raft_service_rs::server::RaftControlClient;
    use raft_service_rs::server::RaftServer;
    use tokio_util::sync::CancellationToken;
    use tonic::async_trait;

    use crate::key_value_service::reading_service::ReadingServiceBuilder;
    use crate::key_value_service::writing_service::WritingServiceBuilder;

    mod writing_service {
        use std::time::Duration;

        use raft_service_rs::LeaderLifetimeService;
        use raft_service_rs::LeaderLifetimeServiceBuilder;
        use raft_service_rs::server::RaftDataClient;
        use tokio::time::sleep;
        use tokio_util::sync::CancellationToken;
        use tonic::async_trait;
        use tracing::info;

        use crate::key_value_service::KeyValueServiceConfig;
        use crate::key_value_service::Request;

        struct WritingService {
            raft_client: RaftDataClient<KeyValueServiceConfig>,
        }

        #[async_trait]
        impl LeaderLifetimeService for WritingService {
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
        }

        impl LeaderLifetimeServiceBuilder for WritingServiceBuilder {
            fn build(&self) -> Box<dyn LeaderLifetimeService> {
                Box::new(WritingService {
                    raft_client: self.raft_client.clone(),
                })
            }
        }
    }

    mod reading_service {
        use std::time::Duration;

        use raft_service_rs::LeaderLifetimeService;
        use raft_service_rs::LeaderLifetimeServiceBuilder;
        use raft_service_rs::server::RaftDataClient;
        use tokio::time::sleep;
        use tokio_util::sync::CancellationToken;
        use tonic::async_trait;
        use tracing::info;

        use crate::key_value_service::KeyValueServiceConfig;

        pub struct ReadingService {
            raft_client: RaftDataClient<KeyValueServiceConfig>,
        }

        #[async_trait]
        impl LeaderLifetimeService for ReadingService {
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
        }

        impl LeaderLifetimeServiceBuilder for ReadingServiceBuilder {
            fn build(&self) -> Box<dyn LeaderLifetimeService> {
                Box::new(ReadingService {
                    raft_client: self.raft_client.clone(),
                })
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

    pub struct KeyValueService {
        raft_control_client: RaftControlClient,
    }

    impl KeyValueService {
        pub async fn new(node_id: u64, port: u16, path: &Path) -> anyhow::Result<Self> {
            let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port);
            let raft_server = RaftServer::new(node_id, addr, path).await?;

            let (mut raft_control_client, raft_data_client) = raft_server.into_client();

            {
                let writing_service = WritingServiceBuilder::new(raft_data_client.clone());

                raft_control_client.register_leader_lifetime_service_builder(writing_service);
            }

            {
                let reading_service = ReadingServiceBuilder::new(raft_data_client);

                raft_control_client.register_leader_lifetime_service_builder(reading_service);
            }

            Ok(Self {
                raft_control_client,
            })
        }

        pub async fn initialize(&self, members: HashMap<u64, Node>) -> anyhow::Result<()> {
            self.raft_control_client.initialize(members).await?;

            Ok(())
        }

        pub async fn add_learner(&self, node: Node) -> anyhow::Result<()> {
            self.raft_control_client
                .add_learner(node.node_id, node)
                .await?;

            Ok(())
        }

        pub async fn add_voter(&self, node: Node) -> anyhow::Result<()> {
            self.raft_control_client.add_voter(node).await?;

            Ok(())
        }

        pub async fn run(&self) -> anyhow::Result<()> {
            let shutdown = CancellationToken::new();

            self.raft_control_client.run(shutdown).await
        }
    }
}
