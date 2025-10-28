use prost::DecodeError;
use serde::Serialize;
use serde::de::DeserializeOwned;
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
    type Request: Serialize + DeserializeOwned + Send + Sync + 'static;

    type ApplicationSnapshot: Serialize + DeserializeOwned + Send + Sync + 'static;

    fn export(&self) -> Self::ApplicationSnapshot;
    fn import(snapshot: Self::ApplicationSnapshot) -> Result<Self, DecodeError>;
    async fn apply(&mut self, request: Self::Request) -> anyhow::Result<bool>;
}

#[async_trait]
pub trait LeaderLifetimeService: Send + Sync + 'static {
    fn on_leader_start(&self) -> anyhow::Result<()>;
    fn on_leader_stop(&self) -> anyhow::Result<()>;
}

pub trait ApplicationConfig: Default + Send + Sync + 'static {
    type Data: ApplicationData;
}
