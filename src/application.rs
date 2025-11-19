use std::fmt::Debug;
use std::fmt::Display;

use prost::DecodeError;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio_util::sync::CancellationToken;
use tonic::async_trait;

use crate::server::RaftDataClient;

pub trait ApplicationConfig: Debug + Copy + Default + Ord + Send + Sync + 'static {
    type Request: Debug + Display + Serialize + DeserializeOwned + Send + Sync + 'static;
    type Response: Serialize + DeserializeOwned + Send + Sync + 'static;
    type Snapshot: Clone + Default + Serialize + DeserializeOwned + Send + Sync;
}

#[async_trait]
pub trait ApplicationStateMachine: Default + Send + Sync + 'static {
    type C: ApplicationConfig;

    fn export(&self) -> <Self::C as ApplicationConfig>::Snapshot;
    fn import(snapshot: <Self::C as ApplicationConfig>::Snapshot) -> Result<Self, DecodeError>;
    async fn apply(
        &mut self,
        request: <Self::C as ApplicationConfig>::Request,
    ) -> anyhow::Result<<Self::C as ApplicationConfig>::Response>;
}

#[async_trait]
pub trait ApplicationLayer: Sized + Send + Sync + 'static {
    type R: ApplicationStateMachine;
    type Config;

    async fn new(
        config: Self::Config,
        sm: RaftDataClient<Self::R>,
        shutdown: CancellationToken,
    ) -> anyhow::Result<Self>;

    async fn leader_lifecycle_start(&mut self) -> anyhow::Result<()>;

    async fn leader_lifecycle_stop(&mut self) -> anyhow::Result<()>;

    async fn shutdown(self) -> anyhow::Result<()>;
}

#[async_trait]
pub trait LeaderLifecycleService: Send {
    async fn run(&self, cancel: CancellationToken);
}

pub trait LeaderLifecycleServiceBuilder {
    fn build(&self) -> Box<dyn LeaderLifecycleService>;
}
