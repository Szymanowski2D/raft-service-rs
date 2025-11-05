use crate::raft::config::type_config::CheckIsLeaderError;
use crate::raft::config::type_config::ClientWriteError;
use crate::raft::config::type_config::RaftError;

pub type ReadError = RaftError<CheckIsLeaderError>;
pub type WriteError = RaftError<ClientWriteError>;
