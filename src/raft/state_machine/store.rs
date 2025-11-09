use std::convert::identity;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use openraft::entry::RaftEntry;
use openraft::storage::RaftStateMachine;
use tokio::sync::RwLock;

use crate::application::ApplicationConfig;
use crate::application::ApplicationData;
use crate::raft::config::type_config::Entry;
use crate::raft::config::type_config::LogId;
use crate::raft::config::type_config::Snapshot;
use crate::raft::config::type_config::SnapshotData;
use crate::raft::config::type_config::SnapshotMeta;
use crate::raft::config::type_config::StorageError;
use crate::raft::config::type_config::StoredMembership;
use crate::raft::config::type_config::TypeConfig;
use crate::raft::state_machine::data::StateMachineData;
use crate::raft::state_machine::snapshot::StoredSnapshot;

#[derive(Default)]
pub(crate) struct StateMachineStore<C: ApplicationConfig> {
    pub(crate) state_machine: RwLock<StateMachineData<C>>,
    pub(super) snapshot_idx: AtomicU64,
    pub(super) current_snapshot: RwLock<Option<StoredSnapshot>>,
}

impl<C: ApplicationConfig> RaftStateMachine<TypeConfig> for Arc<StateMachineStore<C>> {
    type SnapshotBuilder = Self;

    async fn applied_state(&mut self) -> Result<(Option<LogId>, StoredMembership), StorageError> {
        let state_machine = self.state_machine.read().await;

        Ok((
            state_machine.last_applied_log,
            state_machine.last_membership.clone(),
        ))
    }

    async fn apply<I>(&mut self, entries: I) -> Result<Vec<bool>, StorageError>
    where
        I: IntoIterator<Item = Entry>,
    {
        let mut res = Vec::new();

        let mut state_machine = self.state_machine.write().await;

        for entry in entries {
            let log_id = entry.log_id();

            state_machine.last_applied_log = Some(log_id);

            let result = if let Some(req) = entry.app_data {
                let req = prost::Message::decode(req.as_slice())
                    .map_err(|e| StorageError::apply(log_id, &e))?;

                let response = state_machine.application_data.apply(req).await;

                response.is_ok_and(identity)
            } else if let Some(membership) = entry.membership {
                state_machine.last_membership =
                    StoredMembership::new(Some(log_id), membership.into());

                true
            } else {
                true
            };

            res.push(result);
        }

        Ok(res)
    }

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        self.clone()
    }

    async fn begin_receiving_snapshot(&mut self) -> Result<SnapshotData, StorageError> {
        Ok(Default::default())
    }

    async fn install_snapshot(
        &mut self,
        meta: &SnapshotMeta,
        snapshot: SnapshotData,
    ) -> Result<(), StorageError> {
        let new_snapshot = StoredSnapshot {
            meta: meta.clone(),
            data: snapshot,
        };

        // Install state_machine
        {
            let snapshot = prost::Message::decode(new_snapshot.data.as_slice()).map_err(|e| {
                StorageError::read_snapshot(Some(new_snapshot.meta.signature()), &e)
            })?;
            let application_data = C::Data::import(snapshot).map_err(|e| {
                StorageError::read_snapshot(Some(new_snapshot.meta.signature()), &e)
            })?;

            *self.state_machine.write().await = StateMachineData {
                last_applied_log: meta.last_log_id,
                last_membership: meta.last_membership.clone(),
                application_data,
            };
        }

        // Install snapshot
        {
            *self.current_snapshot.write().await = Some(new_snapshot);
        }

        Ok(())
    }

    async fn get_current_snapshot(&mut self) -> Result<Option<Snapshot>, StorageError> {
        match &*self.current_snapshot.read().await {
            Some(snapshot) => Ok(Some(Snapshot {
                meta: snapshot.meta.clone(),
                snapshot: snapshot.data.clone(),
            })),
            None => Ok(None),
        }
    }
}
