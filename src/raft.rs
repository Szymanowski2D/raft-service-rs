use std::path::Path;
use std::sync::Arc;

use openraft::Config;

use crate::ApplicationConfig;
use crate::raft::config::type_config::NodeId;
use crate::raft::config::type_config::Raft;
use crate::raft::log_store::rocksdb::RocksLogStore;
use crate::raft::network::Network;
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

    let network = Network::default();
    let log_store = RocksLogStore::new(path)?;
    let state_machine = Arc::new(StateMachineStore::<C>::default());

    Ok(openraft::Raft::new(node_id, config, network, log_store, state_machine).await?)
}
