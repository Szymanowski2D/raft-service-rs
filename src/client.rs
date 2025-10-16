use std::marker::PhantomData;
use std::path::Path;

use crate::ApplicationConfig;
use crate::grpc::raft_service::RaftServiceImpl;
use crate::raft::config::type_config::Raft;
use crate::raft::new_raft;

pub struct RaftClient<C: ApplicationConfig> {
    raft: Raft,
    _mark: PhantomData<C>,
}

impl<C: ApplicationConfig> RaftClient<C> {
    pub async fn new(node_id: u64, path: &Path) -> anyhow::Result<Self> {
        let raft = new_raft::<C>(node_id, path).await?;

        Ok(Self {
            raft,
            _mark: PhantomData,
        })
    }

    pub fn raft_service(&self) -> RaftServiceImpl {
        RaftServiceImpl::new(self.raft.clone())
    }
}
