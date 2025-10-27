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

    loop {}
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
    use raft_service_rs::server::RaftServer;
    use serde::Deserialize;
    use serde::Serialize;
    use tokio_util::sync::CancellationToken;
    use tonic::async_trait;

    #[derive(Serialize, Deserialize)]
    pub struct Request {
        key: String,
        value: String,
    }

    #[derive(Serialize, Deserialize)]
    pub struct Snapshot {
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

    #[derive(Default)]
    pub struct KeyValueServiceConfig;

    impl ApplicationConfig for KeyValueServiceConfig {
        type Data = KeyValueData;
    }

    pub struct KeyValueService {
        raft_server: RaftServer<KeyValueServiceConfig>,
    }

    impl KeyValueService {
        pub async fn new(node_id: u64, port: u16, path: &Path) -> anyhow::Result<Self> {
            let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port);
            let raft_server = RaftServer::new(node_id, addr, path).await?;

            Ok(Self { raft_server })
        }

        pub async fn initialize(&self, members: HashMap<u64, Node>) -> anyhow::Result<()> {
            self.raft_server.initialize(members).await?;

            Ok(())
        }

        pub async fn add_learner(&self, node: Node) -> anyhow::Result<()> {
            self.raft_server.add_learner(node.node_id, node).await?;

            Ok(())
        }

        pub async fn add_voter(&self, node: Node) -> anyhow::Result<()> {
            self.raft_server.add_voter(node).await?;

            Ok(())
        }

        pub async fn run(&self) -> anyhow::Result<()> {
            let shutdown = CancellationToken::new();

            self.raft_server.run(shutdown).await
        }
    }
}
