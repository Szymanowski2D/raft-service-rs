use crate::application::ApplicationStateMachine;
use crate::raft::config::type_config::LogId;
use crate::raft::config::type_config::StoredMembership;

#[derive(Default)]
pub(crate) struct StateMachineData<A: ApplicationStateMachine> {
    pub(super) last_applied_log: Option<LogId<A::C>>,
    pub(super) last_membership: StoredMembership<A::C>,
    pub(crate) application_data: A,
}
