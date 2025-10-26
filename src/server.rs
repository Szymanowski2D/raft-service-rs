use std::collections::HashMap;
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::path::Path;

use futures_util::future::join_all;
use tokio_util::sync::CancellationToken;
use tonic::transport::Server;
use tracing::trace;

use crate::ApplicationConfig;
use crate::LeaderLifetimeService;
use crate::grpc::raft_service::RaftServiceImpl;
use crate::pb::raft_service_server::RaftServiceServer;
use crate::raft::config::type_config::Node;
use crate::raft::config::type_config::NodeId;
use crate::raft::config::type_config::Raft;
use crate::raft::new_raft;

pub struct RaftServer<C: ApplicationConfig> {
    node_id: u64,
    listening: SocketAddr,
    raft: Raft,
    leader_lifetime_services: Vec<Box<dyn LeaderLifetimeService>>,
    _mark: PhantomData<C>,
}

impl<C: ApplicationConfig> RaftServer<C> {
    pub async fn new(
        node_id: u64,
        listening: SocketAddr,
        log_store_path: &Path,
    ) -> anyhow::Result<Self> {
        let raft = new_raft::<C>(node_id, log_store_path).await?;

        Ok(Self {
            node_id,
            listening,
            raft,
            leader_lifetime_services: Vec::new(),
            _mark: PhantomData,
        })
    }

    pub async fn initialize(&self, members: HashMap<NodeId, Node>) -> anyhow::Result<()> {
        self.raft.initialize(members).await?;

        Ok(())
    }

    pub async fn add_learner(&self, id: NodeId, node: Node) -> anyhow::Result<()> {
        self.raft.add_learner(id, node, true).await?;

        Ok(())
    }

    pub async fn register_leader_lifetime_services(
        &mut self,
        service: Box<dyn LeaderLifetimeService>,
    ) {
        self.leader_lifetime_services.push(service);
    }

    pub async fn run(&self, shutdown: CancellationToken) -> anyhow::Result<()> {
        trace!(self.node_id, "server is running");

        let mut metrics_rx = self.raft.metrics();

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

        let mut is_active = false;

        while !shutdown.is_cancelled() {
            tokio::select! {
                biased;

                _ = shutdown.cancelled() => break,
                _ = metrics_rx.changed() => {}
            }

            let is_leader = {
                let metrics = metrics_rx.borrow_and_update();

                metrics.state.is_leader()
            };

            if is_leader {
                if !is_active {
                    trace!(self.node_id, "Node becomes a leader");

                    let futures: Vec<_> = self
                        .leader_lifetime_services
                        .iter()
                        .map(|service| service.on_leader_start())
                        .collect();

                    join_all(futures).await;

                    is_active = true;
                }
            } else if is_active {
                trace!(self.node_id, "Node is not a leader");

                let futures: Vec<_> = self
                    .leader_lifetime_services
                    .iter()
                    .map(|service| service.on_leader_stop())
                    .collect();

                join_all(futures).await;

                is_active = false;
            }
        }

        if !is_active {
            let futures: Vec<_> = self
                .leader_lifetime_services
                .iter()
                .map(|service| service.on_leader_stop())
                .collect();

            join_all(futures).await;
        }

        raft_service_handle.await?;

        Ok(())
    }
}
