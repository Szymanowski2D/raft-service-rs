use prost::DecodeError;
use tonic::async_trait;

pub mod server;

pub(crate) mod grpc;
pub(crate) mod raft;

mod pb {
    tonic::include_proto!("openraftpb");
}

pub use pb::Node;

#[async_trait]
pub trait ApplicationData: Default + Send + Sync {
    type Request: prost::Message + Default + Send + Sync + 'static;

    type ApplicationSnapshot: prost::Message + Default + Send + Sync + 'static;

    fn export(&self) -> Self::ApplicationSnapshot;
    fn import(snapshot: Self::ApplicationSnapshot) -> Result<Self, DecodeError>;
    async fn apply(&mut self, request: Self::Request) -> anyhow::Result<bool>;
}

#[async_trait]
pub trait LeaderLifetimeService: Send + Sync + 'static {
    async fn start(&self) -> anyhow::Result<()>;
    async fn stop(&self) -> anyhow::Result<()>;
}

pub trait LeaderLifetimeServiceBuilder: Send + Sync + 'static {
    fn build(&self) -> anyhow::Result<Box<dyn LeaderLifetimeService>>;
}

pub trait ApplicationConfig: Default + Send + Sync + 'static {
    type Data: ApplicationData;
}
