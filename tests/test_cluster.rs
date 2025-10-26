use std::time::Duration;

use tempfile::NamedTempFile;

use crate::key_value_service::KeyValueService;

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn test_cluster() -> anyhow::Result<()> {
    let _s1 = tokio::spawn(async {
        let tmpfile = NamedTempFile::new()?;

        let raft_server = KeyValueService::new(1, 21001, tmpfile.path()).await?;
        raft_server.run().await?;

        anyhow::Ok(())
    });

    let _s2 = tokio::spawn(async {
        let tmpfile = NamedTempFile::new()?;

        let raft_server = KeyValueService::new(2, 21002, tmpfile.path()).await?;
        raft_server.run().await?;

        anyhow::Ok(())
    });

    let _s3 = tokio::spawn(async {
        let tmpfile = NamedTempFile::new()?;

        let raft_server = KeyValueService::new(3, 21003, tmpfile.path()).await?;
        raft_server.run().await?;

        anyhow::Ok(())
    });

    // Wait for server to start up.
    tokio::time::sleep(Duration::from_millis(200)).await;

    loop {}
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

        pub async fn run(&self) -> anyhow::Result<()> {
            let shutdown = CancellationToken::new();

            self.raft_server.run(shutdown).await
        }
    }
}
