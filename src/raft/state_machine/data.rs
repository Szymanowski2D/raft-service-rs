use crate::ApplicationConfig;
use crate::raft::config::type_config::LogId;
use crate::raft::config::type_config::StoredMembership;

#[derive(Default)]
pub(crate) struct StateMachineData<C: ApplicationConfig> {
    pub(super) last_applied_log: Option<LogId>,
    pub(super) last_membership: StoredMembership,
    pub(crate) application_data: C::Data,
}
