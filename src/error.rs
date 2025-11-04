use crate::raft::config::type_config::{CheckIsLeaderError, ClientWriteError, RaftError};

pub type ReadError = RaftError<CheckIsLeaderError>;
pub type WriteError = RaftError<ClientWriteError>;
