use crate::pb::ClientWriteResponse as PbClientWriteResponse;
use crate::raft::config::type_config::ClientWriteResponse;

impl From<ClientWriteResponse> for PbClientWriteResponse {
    fn from(resp: ClientWriteResponse) -> Self {
        PbClientWriteResponse {
            log_id: Some(resp.log_id.into()),
            membership: resp.membership.map(Into::into),
        }
    }
}
