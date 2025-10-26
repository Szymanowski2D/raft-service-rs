use std::marker::PhantomData;
use std::net::SocketAddr;
use std::path::Path;

use futures_util::future::join_all;
use tokio_util::sync::CancellationToken;
use tonic::transport::Server;

use crate::ApplicationConfig;
use crate::LeaderLifetimeService;
use crate::grpc::raft_service::RaftServiceImpl;
use crate::pb::raft_service_server::RaftServiceServer;
use crate::raft::config::type_config::Raft;
use crate::raft::new_raft;

pub struct RaftServer<C: ApplicationConfig> {
    raft: Raft,
    listening: SocketAddr,
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
            raft,
            listening,
            leader_lifetime_services: Vec::new(),
            _mark: PhantomData,
        })
    }

    pub async fn register_leader_lifetime_services(
        &mut self,
        service: Box<dyn LeaderLifetimeService>,
    ) {
        self.leader_lifetime_services.push(service);
    }

    pub async fn run(&self, shutdown: CancellationToken) -> anyhow::Result<()> {
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

            let is_leader = metrics_rx.borrow().state.is_leader();

            if is_leader {
                if is_active {
                    let futures: Vec<_> = self
                        .leader_lifetime_services
                        .iter()
                        .map(|service| service.on_leader_start())
                        .collect();

                    join_all(futures).await;

                    is_active = true;
                }
            } else if !is_active {
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
