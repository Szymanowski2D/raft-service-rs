use std::fmt::Display;

use openraft::entry::RaftEntry;
use openraft::entry::RaftPayload;
use prost::Message;

use crate::application::ApplicationConfig;
use crate::pb::internal::Entry;
use crate::raft::config::type_config::EntryPayload;
use crate::raft::config::type_config::LogId;
use crate::raft::config::type_config::Membership;
use crate::raft::config::type_config::TypeConfig;

impl Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Entry{{term={},index={}}}", self.term, self.index)
    }
}

impl<C: ApplicationConfig> RaftPayload<TypeConfig<C>> for Entry {
    fn get_membership(&self) -> Option<Membership<C>> {
        self.membership.clone().map(Into::into)
    }
}

impl<C: ApplicationConfig> RaftEntry<TypeConfig<C>> for Entry {
    fn new(log_id: LogId<C>, payload: EntryPayload<C>) -> Self {
        let mut app_data = None;
        let mut membership = None;
        match payload {
            EntryPayload::Blank => {}
            EntryPayload::Normal(data) => app_data = Some(data.encode_to_vec()),
            EntryPayload::Membership(m) => membership = Some(m.into()),
        }

        Self {
            term: log_id.leader_id,
            index: log_id.index,
            app_data,
            membership,
        }
    }

    fn log_id_parts(&self) -> (&u64, u64) {
        (&self.term, self.index)
    }

    fn set_log_id(&mut self, new: LogId<C>) {
        self.term = new.leader_id;
        self.index = new.index;
    }
}
