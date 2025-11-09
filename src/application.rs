use prost::DecodeError;
use tokio_util::sync::CancellationToken;
use tonic::async_trait;

use crate::server::RaftDataClient;

#[async_trait]
pub trait ApplicationData: Default + Send + Sync {
    type Request: prost::Message + Default + Send + Sync + 'static;

    type ApplicationSnapshot: prost::Message + Default + Send + Sync + 'static;

    fn export(&self) -> Self::ApplicationSnapshot;
    fn import(snapshot: Self::ApplicationSnapshot) -> Result<Self, DecodeError>;
    async fn apply(&mut self, request: Self::Request) -> anyhow::Result<bool>;
}

pub trait ApplicationConfig: Default + Send + Sync + 'static {
    type Data: ApplicationData;
}

#[async_trait]
pub trait ApplicationLayer: Sized + Send + Sync + 'static {
    type C: ApplicationConfig;
    type Config;

    async fn new(config: Self::Config, sm: RaftDataClient<Self::C>) -> anyhow::Result<Self>;

    async fn start(&mut self, cancel: CancellationToken) -> anyhow::Result<()>;

    async fn wait_to_stop(&mut self) -> anyhow::Result<()>;

    async fn shutdown(self) -> anyhow::Result<()>;
}
