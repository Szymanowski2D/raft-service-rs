use std::path::Path;
use std::sync::Arc;

use openraft::Config;

use crate::ApplicationConfig;
use crate::raft::config::type_config::NodeId;
use crate::raft::config::type_config::Raft;
use crate::raft::log_store::rocksdb::RocksLogStore;
use crate::raft::network::grpc::GRPCNetwork;
use crate::raft::state_machine::store::StateMachineStore;

pub(crate) mod config;
pub(crate) mod state_machine;

mod log_store;
mod network;
mod pb_impl;

pub async fn new_raft<C: ApplicationConfig>(node_id: NodeId, path: &Path) -> anyhow::Result<Raft> {
    let config = Arc::new(
        Config {
            heartbeat_interval: 500,
            election_timeout_min: 1500,
            election_timeout_max: 3000,
            ..Default::default()
        }
        .validate()?,
    );

    let network = GRPCNetwork::default();
    let log_store = RocksLogStore::new(path)?;
    let state_machine = Arc::new(StateMachineStore::<C>::default());

    Ok(openraft::Raft::new(node_id, config, network, log_store, state_machine).await?)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use openraft::testing::log::StoreBuilder;
    use openraft::testing::log::Suite;
    use prost::DecodeError;
    use tempfile::TempDir;
    use tonic::async_trait;

    use crate::ApplicationConfig;
    use crate::ApplicationData;
    use crate::raft::RocksLogStore;
    use crate::raft::config::type_config::StorageError;
    use crate::raft::config::type_config::TypeConfig;
    use crate::raft::state_machine::store::StateMachineStore;

    #[derive(Default)]
    struct MockApplicationData {}

    #[async_trait]
    impl ApplicationData for MockApplicationData {
        type Request = ();

        type ApplicationSnapshot = ();

        fn export(&self) -> Self::ApplicationSnapshot {
            ()
        }

        fn import(_snapshot: &Self::ApplicationSnapshot) -> Result<Self, DecodeError> {
            Ok(MockApplicationData::default())
        }

        async fn apply(&mut self, _request: Self::Request) -> anyhow::Result<bool> {
            Ok(true)
        }
    }

    #[derive(Default)]
    struct MockApplicationConfig {}

    impl ApplicationConfig for MockApplicationConfig {
        type ApplicationData = MockApplicationData;
    }

    struct RocksStoreBuilder {}

    impl
        StoreBuilder<
            TypeConfig,
            RocksLogStore<TypeConfig>,
            Arc<StateMachineStore<MockApplicationConfig>>,
            (),
        > for RocksStoreBuilder
    {
        async fn build(
            &self,
        ) -> Result<
            (
                (),
                RocksLogStore<TypeConfig>,
                Arc<StateMachineStore<MockApplicationConfig>>,
            ),
            StorageError,
        > {
            let tmp_dir = TempDir::new().map_err(|e| StorageError::read_logs(&e))?;

            Ok((
                (),
                RocksLogStore::new(tmp_dir.path()).map_err(|e| StorageError::read_logs(&e))?,
                Arc::default(),
            ))
        }
    }

    #[tokio::test]
    async fn test_store_with_rocks() -> anyhow::Result<()> {
        Suite::test_all(RocksStoreBuilder {}).await?;

        Ok(())
    }
}
