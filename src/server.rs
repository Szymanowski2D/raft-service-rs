use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use maplit::btreemap;
use openraft::ChangeMembers;
use prost::Message;
use serde::Deserialize;
use tracing::info;

use crate::application::ApplicationConfig;
use crate::application::ApplicationData;
use crate::raft::config::type_config::CheckIsLeaderError;
use crate::raft::config::type_config::ClientWriteError;
use crate::raft::config::type_config::ClientWriteResponse;
use crate::raft::config::type_config::Node;
use crate::raft::config::type_config::NodeId;
use crate::raft::config::type_config::Raft;
use crate::raft::config::type_config::RaftError;
use crate::raft::new_raft;
use crate::raft::state_machine::store::StateMachineStore;

#[derive(Clone, Debug, Deserialize)]
pub struct RaftServiceConfig {
    pub node_id: u64,
    pub rpc_url: String,
    pub log_path: PathBuf,
}

#[derive(Clone)]
pub struct RaftDataClient<C: ApplicationConfig> {
    node_id: u64,
    state_machine: Arc<StateMachineStore<C>>,
    raft: Raft,
}

impl<C: ApplicationConfig> RaftDataClient<C> {
    pub fn node_id(&self) -> u64 {
        self.node_id
    }

    pub async fn write(
        &self,
        request: &<C::Data as ApplicationData>::Request,
    ) -> Result<ClientWriteResponse, RaftError<ClientWriteError>> {
        let buf = request.encode_to_vec();
        self.raft.client_write(buf).await
    }

    pub async fn read<R>(&self, f: impl FnOnce(&C::Data) -> R) -> R {
        let sm = self.state_machine.state_machine.read().await;

        f(&sm.application_data)
    }

    pub async fn read_safe<R>(
        &self,
        f: impl FnOnce(&C::Data) -> R,
    ) -> Result<R, RaftError<CheckIsLeaderError>> {
        let ret = self
            .raft
            .get_read_linearizer(openraft::ReadPolicy::ReadIndex)
            .await;

        match ret {
            Ok(linearizer) => {
                linearizer.await_ready(&self.raft).await.unwrap();

                let sm = self.state_machine.state_machine.read().await;

                Ok(f(&sm.application_data))
            }
            Err(err) => Err(err),
        }
    }
}

pub struct RaftControlClient {
    node_id: u64,
    raft: Raft,
}

impl RaftControlClient {
    pub fn node_id(&self) -> u64 {
        self.node_id
    }

    pub async fn is_initialized(&self) -> anyhow::Result<bool> {
        Ok(self.raft.is_initialized().await?)
    }

    pub async fn initialize(&self, members: HashMap<NodeId, Node>) -> anyhow::Result<()> {
        self.raft.initialize(members).await?;

        Ok(())
    }

    pub async fn add_learner(&self, id: NodeId, node: Node) -> anyhow::Result<()> {
        self.raft.add_learner(id, node, true).await?;

        Ok(())
    }

    pub async fn add_voter(&self, node: Node) -> anyhow::Result<()> {
        self.raft
            .change_membership(
                ChangeMembers::AddVoters(btreemap! {
                    node.node_id => node
                }),
                false,
            )
            .await?;

        Ok(())
    }
}

pub struct RaftServer<C: ApplicationConfig> {
    pub(crate) node_id: u64,
    pub(crate) state_machine: Arc<StateMachineStore<C>>,
    pub(crate) raft: Raft,
}

impl<C: ApplicationConfig> RaftServer<C> {
    pub async fn new_from_config(config: &RaftServiceConfig) -> anyhow::Result<Self> {
        Self::new(config.node_id, &config.log_path).await
    }

    pub async fn new(node_id: u64, log_store_path: &Path) -> anyhow::Result<Self> {
        info!(node_id, ?log_store_path, "Init raft");

        let (state_machine, raft) = new_raft::<C>(node_id, log_store_path).await?;

        Ok(Self {
            node_id,
            state_machine,
            raft,
        })
    }

    pub fn control_client(&self) -> RaftControlClient {
        RaftControlClient {
            node_id: self.node_id,
            raft: self.raft.clone(),
        }
    }

    pub fn data_client(&self) -> RaftDataClient<C> {
        RaftDataClient {
            node_id: self.node_id,
            state_machine: self.state_machine.clone(),
            raft: self.raft.clone(),
        }
    }
}
