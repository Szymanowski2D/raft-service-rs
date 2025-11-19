use crate::application::ApplicationConfig;
use crate::pb::internal::VoteRequest as PbVoteRequest;
use crate::raft::config::type_config::VoteRequest;

impl<C: ApplicationConfig> From<VoteRequest<C>> for PbVoteRequest {
    fn from(req: VoteRequest<C>) -> Self {
        Self {
            vote: Some(req.vote),
            last_log_id: req.last_log_id.map(Into::into),
        }
    }
}

impl<C: ApplicationConfig> From<PbVoteRequest> for VoteRequest<C> {
    fn from(req: PbVoteRequest) -> Self {
        Self {
            vote: req.vote.unwrap(),
            last_log_id: req.last_log_id.map(Into::into),
        }
    }
}
