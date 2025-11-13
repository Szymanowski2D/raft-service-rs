use anyhow::Context;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tonic::transport::Server;
use tracing::debug;
use tracing::error;

use crate::application::ApplicationLayer;
use crate::grpc::controller_service::RaftControllerServiceImpl;
use crate::grpc::internal_service::RaftServiceImpl;
use crate::pb::controller::raft_controller_service_server::RaftControllerServiceServer;
use crate::pb::internal::raft_service_server::RaftServiceServer;
use crate::server::RaftServer;
use crate::server::RaftServiceConfig;

pub struct RaftOrchestrator<A: ApplicationLayer> {
    application_config: A::Config,
    raft_config: RaftServiceConfig,
    raft_server: RaftServer<A::C>,
}

impl<A> RaftOrchestrator<A>
where
    A: ApplicationLayer,
{
    pub async fn new(
        raft_config: RaftServiceConfig,
        application_config: A::Config,
    ) -> anyhow::Result<Self> {
        let raft_server = RaftServer::new_from_config(&raft_config)
            .await
            .context("Failed to create raft_server")?;

        Ok(RaftOrchestrator {
            application_config,
            raft_config,
            raft_server,
        })
    }

    pub async fn run(self, shutdown_token: CancellationToken) -> anyhow::Result<()> {
        let data_client = self.raft_server.data_client();
        let raft = self.raft_server.raft;

        let mut join_set = JoinSet::new();

        // Raft internal service and control service
        join_set.spawn({
            let shutdown_token = shutdown_token.clone();
            let addr = self.raft_config.rpc_url.parse()?;
            let raft = raft.clone();
            async move {
                Server::builder()
                    .add_service(RaftServiceServer::new(RaftServiceImpl::new(raft.clone())))
                    .add_service(RaftControllerServiceServer::new(
                        RaftControllerServiceImpl::new(raft),
                    ))
                    .serve_with_shutdown(addr, shutdown_token.cancelled())
                    .await?;

                anyhow::Ok(())
            }
        });

        // Raft leader status changing
        {
            let mut application =
                A::new(self.application_config, data_client, shutdown_token.clone()).await?;

            join_set.spawn({
                let shutdown_token = shutdown_token.clone();
                let metrics_rx = raft.server_metrics();
                async move {
                    let mut metrics_rx = metrics_rx;

                    let mut stop_token = None;

                    while !shutdown_token.is_cancelled() {
                        tokio::select! {
                            biased;

                            _ = shutdown_token.cancelled() => break,
                            _ = metrics_rx.changed() => {}
                        }

                        let metrics = metrics_rx.borrow_and_update().clone();

                        debug!(metrics.id, ?metrics, "Metrics changed");

                        if metrics.state.is_leader() {
                            if stop_token.is_none() {
                                debug!(metrics.id, "Node becomes a leader");

                                let token = CancellationToken::new();
                                application.leader_lifetime_start(token.clone()).await?;
                                stop_token = Some(token);
                            }
                        } else if let Some(token) = stop_token.take() {
                            token.cancel();
                            application.leader_lifetime_stop().await?;
                        }
                    }

                    if let Some(token) = stop_token.take() {
                        token.cancel();
                        application.leader_lifetime_stop().await?;
                    }

                    application.shutdown().await?;

                    Ok(())
                }
            });
        }

        shutdown_token.cancelled().await;
        while let Some(res) = join_set.join_next().await {
            if let Err(err) = res {
                error!(?err, "Background task exited unexpectedly");
            }
        }
        raft.shutdown().await?;

        Ok(())
    }
}
