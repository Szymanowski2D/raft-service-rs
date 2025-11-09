use crate::pb::common::LogId as PbLogId;
use crate::raft::config::type_config::LogId;

impl From<LogId> for PbLogId {
    fn from(log_id: LogId) -> Self {
        Self {
            term: *log_id.committed_leader_id(),
            index: log_id.index,
        }
    }
}

impl From<PbLogId> for LogId {
    fn from(log_id: PbLogId) -> Self {
        Self {
            leader_id: log_id.term,
            index: log_id.index,
        }
    }
}
