use crate::pb::VoteRequest as PbVoteRequest;
use crate::raft::config::type_config::VoteRequest;

impl From<VoteRequest> for PbVoteRequest {
    fn from(req: VoteRequest) -> Self {
        Self {
            vote: Some(req.vote),
            last_log_id: req.last_log_id.map(Into::into),
        }
    }
}

impl From<PbVoteRequest> for VoteRequest {
    fn from(req: PbVoteRequest) -> Self {
        Self {
            vote: req.vote.unwrap(),
            last_log_id: req.last_log_id.map(Into::into),
        }
    }
}
