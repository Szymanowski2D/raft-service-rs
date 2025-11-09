use std::fmt;

use openraft::vote::RaftLeaderId;

use crate::pb::internal::LeaderId;
use crate::raft::config::type_config::NodeId;
use crate::raft::config::type_config::Term;
use crate::raft::config::type_config::TypeConfig;

impl fmt::Display for LeaderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "T{}-N{}", self.term, self.node_id)
    }
}

impl RaftLeaderId<TypeConfig> for LeaderId {
    type Committed = u64;

    fn new(term: Term, node_id: NodeId) -> Self {
        Self { term, node_id }
    }

    fn term(&self) -> Term {
        self.term
    }

    fn node_id(&self) -> Option<&NodeId> {
        Some(&self.node_id)
    }

    fn to_committed(&self) -> Self::Committed {
        self.term
    }
}
