use crate::ApplicationConfig;
use crate::raft::config::type_config::LogId;
use crate::raft::config::type_config::StoredMembership;

#[derive(Default)]
pub(super) struct StateMachineData<C: ApplicationConfig> {
    pub(super) last_applied_log: Option<LogId>,
    pub(super) last_membership: StoredMembership,
    pub(super) application_data: C::Data,
}
