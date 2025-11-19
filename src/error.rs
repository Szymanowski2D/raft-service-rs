use crate::raft::config::type_config::CheckIsLeaderError;
use crate::raft::config::type_config::ClientWriteError;
use crate::raft::config::type_config::RaftError;

pub type ReadError<C> = RaftError<C, CheckIsLeaderError<C>>;
pub type WriteError<C> = RaftError<C, ClientWriteError<C>>;
