use std::fmt;

use openraft::vote::RaftVote;

use crate::pb::Vote as PbVote;
use crate::raft::config::type_config::LeaderId;
use crate::raft::config::type_config::TypeConfig;

impl RaftVote<TypeConfig> for PbVote {
    fn from_leader_id(leader_id: LeaderId, committed: bool) -> Self {
        Self {
            leader_id: Some(leader_id),
            committed,
        }
    }

    fn leader_id(&self) -> Option<&LeaderId> {
        self.leader_id.as_ref()
    }

    fn is_committed(&self) -> bool {
        self.committed
    }
}

impl fmt::Display for PbVote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "<{}:{}>",
            self.leader_id.as_ref().unwrap_or(&Default::default()),
            if self.committed { "Q" } else { "-" }
        )
    }
}
