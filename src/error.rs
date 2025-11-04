use crate::raft::config::type_config::{CheckIsLeaderError, RaftError};

pub type ReadError = RaftError<CheckIsLeaderError>;
