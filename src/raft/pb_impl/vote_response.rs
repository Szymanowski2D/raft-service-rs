use crate::pb::internal::VoteResponse as PbVoteResponse;
use crate::raft::config::type_config::VoteResponse;

impl From<PbVoteResponse> for VoteResponse {
    fn from(resp: PbVoteResponse) -> Self {
        VoteResponse::new(
            resp.vote.unwrap(),
            resp.last_log_id.map(Into::into),
            resp.vote_granted,
        )
    }
}

impl From<VoteResponse> for PbVoteResponse {
    fn from(resp: VoteResponse) -> Self {
        Self {
            vote: Some(resp.vote),
            vote_granted: resp.vote_granted,
            last_log_id: resp.last_log_id.map(Into::into),
        }
    }
}
