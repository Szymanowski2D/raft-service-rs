use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use futures_util::future::join_all;
use maplit::btreemap;
use openraft::ChangeMembers;
use prost::Message;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;
use tonic::transport::Server;
use tracing::trace;

use crate::ApplicationConfig;
use crate::ApplicationData;
use crate::LeaderLifetimeService;
use crate::LeaderLifetimeServiceBuilder;
use crate::grpc::raft_service::RaftServiceImpl;
use crate::pb::raft_service_server::RaftServiceServer;
use crate::raft::config::type_config::CheckIsLeaderError;
use crate::raft::config::type_config::ClientWriteError;
use crate::raft::config::type_config::ClientWriteResponse;
use crate::raft::config::type_config::Node;
use crate::raft::config::type_config::NodeId;
use crate::raft::config::type_config::Raft;
use crate::raft::config::type_config::RaftError;
use crate::raft::new_raft;
use crate::raft::state_machine::store::StateMachineStore;

#[derive(Debug, Deserialize)]
pub struct RaftServiceConfig {
    pub node_id: u64,
    pub listening: String,
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

struct RaftLeaderLifetimeServiceRuntime {
    services: Vec<Box<dyn LeaderLifetimeService>>,
}

impl RaftLeaderLifetimeServiceRuntime {
    fn new(services: Vec<Box<dyn LeaderLifetimeService>>) -> Self {
        RaftLeaderLifetimeServiceRuntime { services }
    }

    async fn start_all(&self) -> anyhow::Result<()> {
        let mut futures = Vec::with_capacity(self.services.len());
        for service in &self.services {
            futures.push(service.start());
        }
        let _r = join_all(futures).await;

        Ok(())
    }

    async fn stop_all(&self) -> anyhow::Result<()> {
        let mut futures = Vec::with_capacity(self.services.len());
        for service in &self.services {
            futures.push(service.stop());
        }
        let _r = join_all(futures).await;

        Ok(())
    }
}

pub struct RaftControlClient {
    node_id: u64,
    listening: SocketAddr,
    raft: Raft,
    leader_lifetime_service_builders: Vec<Box<dyn LeaderLifetimeServiceBuilder>>,
}

impl RaftControlClient {
    fn build_leader_lifetime_services(&self) -> anyhow::Result<RaftLeaderLifetimeServiceRuntime> {
        let runtime = RaftLeaderLifetimeServiceRuntime::new(
            self.leader_lifetime_service_builders
                .iter()
                .map(|builder| builder.build())
                .collect(),
        );

        Ok(runtime)
    }

    pub fn register_leader_lifetime_service_builder(
        &mut self,
        service: impl LeaderLifetimeServiceBuilder,
    ) {
        self.leader_lifetime_service_builders
            .push(Box::new(service));
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

    pub async fn run(&self, shutdown: CancellationToken) -> anyhow::Result<()> {
        trace!(self.node_id, "server is running");

        let mut metrics_rx = self.raft.server_metrics();

        let raft_service_handle = tokio::spawn({
            let raft = self.raft.clone();
            let addr = self.listening;
            let shutdown = shutdown.clone();

            async move {
                Server::builder()
                    .add_service(RaftServiceServer::new(RaftServiceImpl::new(raft)))
                    .serve_with_shutdown(addr, shutdown.cancelled())
                    .await
                    .unwrap()
            }
        });

        let mut runtime = None;

        while !shutdown.is_cancelled() {
            tokio::select! {
                biased;

                _ = shutdown.cancelled() => break,
                _ = metrics_rx.changed() => {}
            }

            let metrics = metrics_rx.borrow_and_update().clone();

            trace!(self.node_id, ?metrics, "Metrics changed");

            if metrics.state.is_leader() {
                if runtime.is_none() {
                    trace!(self.node_id, "Node becomes a leader");

                    let services = self.build_leader_lifetime_services()?;
                    services.start_all().await?;
                    runtime = Some(services);
                }
            } else if let Some(runtime) = runtime.take() {
                runtime.stop_all().await?;
            }
        }

        if let Some(runtime) = runtime.take() {
            runtime.stop_all().await?;
        }

        raft_service_handle.await?;

        Ok(())
    }
}

pub struct RaftServer<C: ApplicationConfig> {
    node_id: u64,
    listening: SocketAddr,
    state_machine: Arc<StateMachineStore<C>>,
    raft: Raft,
}

impl<C: ApplicationConfig> RaftServer<C> {
    pub async fn new(
        node_id: u64,
        listening: SocketAddr,
        log_store_path: &Path,
    ) -> anyhow::Result<Self> {
        let (state_machine, raft) = new_raft::<C>(node_id, log_store_path).await?;

        Ok(Self {
            node_id,
            listening,
            state_machine,
            raft,
        })
    }

    pub fn into_client(self) -> (RaftControlClient, RaftDataClient<C>) {
        (
            RaftControlClient {
                node_id: self.node_id,
                listening: self.listening,
                raft: self.raft.clone(),
                leader_lifetime_service_builders: Vec::new(),
            },
            RaftDataClient {
                node_id: self.node_id,
                state_machine: self.state_machine,
                raft: self.raft,
            },
        )
    }
}
