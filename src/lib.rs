use std::fmt::Debug;

use prost::DecodeError;
use tonic::async_trait;

pub mod client;

pub(crate) mod grpc;
pub(crate) mod raft;

mod pb {
    tonic::include_proto!("openraftpb");
}

#[async_trait]
pub trait ApplicationData: Default + Send + Sync {
    type Request: prost::Message + Default;

    type ApplicationSnapshot: prost::Message + Default;

    fn export(&self) -> Self::ApplicationSnapshot;
    fn import(snapshot: &Self::ApplicationSnapshot) -> Result<Self, DecodeError>;
    async fn apply(&mut self, request: Self::Request) -> anyhow::Result<bool>;
}

pub trait ApplicationConfig: Copy + Debug + Default + Ord + Send + Sync + 'static {
    type ApplicationData: ApplicationData;
}
