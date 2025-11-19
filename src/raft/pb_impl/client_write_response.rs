use crate::application::ApplicationConfig;
use crate::pb::controller::ClientWriteResponse as PbClientWriteResponse;
use crate::raft::config::type_config::ClientWriteResponse;

impl<C: ApplicationConfig> From<ClientWriteResponse<C>> for PbClientWriteResponse {
    fn from(resp: ClientWriteResponse<C>) -> Self {
        PbClientWriteResponse {
            log_id: Some(resp.log_id.into()),
            membership: resp.membership.map(Into::into),
        }
    }
}
