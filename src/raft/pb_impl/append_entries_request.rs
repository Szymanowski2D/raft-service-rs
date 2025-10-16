use crate::pb::AppendEntriesRequest as PbAppendEntriesRequest;
use crate::raft::config::type_config::AppendEntriesRequest;

impl From<AppendEntriesRequest> for PbAppendEntriesRequest {
    fn from(req: AppendEntriesRequest) -> Self {
        Self {
            vote: Some(req.vote),
            prev_log_id: req.prev_log_id.map(Into::into),
            entries: req.entries,
            leader_commit: req.leader_commit.map(Into::into),
        }
    }
}

impl From<PbAppendEntriesRequest> for AppendEntriesRequest {
    fn from(req: PbAppendEntriesRequest) -> Self {
        Self {
            vote: req.vote.unwrap(),
            prev_log_id: req.prev_log_id.map(Into::into),
            entries: req.entries,
            leader_commit: req.leader_commit.map(Into::into),
        }
    }
}
