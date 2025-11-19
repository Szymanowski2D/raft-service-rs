use std::io;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use futures_util::Stream;
use futures_util::TryStreamExt;
use openraft::AnyError;
use openraft::OptionalSend;
use openraft::entry::RaftEntry;
use openraft::storage::EntryResponder;
use openraft::storage::RaftStateMachine;
use tokio::sync::RwLock;

use crate::application::ApplicationStateMachine;
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
pub(crate) struct StateMachineStore<A: ApplicationStateMachine> {
    pub(crate) state_machine: RwLock<StateMachineData<A>>,
    pub(super) snapshot_idx: AtomicU64,
    pub(super) current_snapshot: RwLock<Option<StoredSnapshot<A::C>>>,
}

impl<A: ApplicationStateMachine> RaftStateMachine<TypeConfig<A::C>> for Arc<StateMachineStore<A>> {
    type SnapshotBuilder = Self;

    async fn applied_state(
        &mut self,
    ) -> Result<(Option<LogId<A::C>>, StoredMembership<A::C>), io::Error> {
        let state_machine = self.state_machine.read().await;

        Ok((
            state_machine.last_applied_log,
            state_machine.last_membership.clone(),
        ))
    }

    async fn apply<Strm>(&mut self, mut entries: Strm) -> Result<(), io::Error>
    where
        Strm: Stream<Item = Result<EntryResponder<TypeConfig<A::C>>, io::Error>>
            + Unpin
            + OptionalSend,
    {
        let mut state_machine = self.state_machine.write().await;

        while let Some((entry, response)) = entries.try_next().await? {
            let log_id = entry.log_id();

            state_machine.last_applied_log = Some(log_id);

            let value = if let Some(req) = entry.app_data {
                let req = serde_json::from_slice(req.as_slice())
                    .map_err(|e| StorageError::apply(log_id, &e))?;

                let response = state_machine
                    .application_data
                    .apply(req)
                    .await
                    .map_err(|e| StorageError::apply(log_id, AnyError::error(e.to_string())))?;

                Some(response)
            } else if let Some(membership) = entry.membership {
                state_machine.last_membership =
                    StoredMembership::new(Some(log_id), membership.into());

                None
            } else {
                None
            };

            if let Some(response) = response
                && let Some(value) = value
            {
                response.send(value);
            }
        }

        Ok(())
    }

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        self.clone()
    }

    async fn begin_receiving_snapshot(&mut self) -> Result<SnapshotData<A::C>, io::Error> {
        Ok(Default::default())
    }

    async fn install_snapshot(
        &mut self,
        meta: &SnapshotMeta<A::C>,
        snapshot: SnapshotData<A::C>,
    ) -> Result<(), io::Error> {
        let new_snapshot = StoredSnapshot {
            meta: meta.clone(),
            data: snapshot,
        };

        // Install state_machine
        {
            let snapshot = serde_json::from_slice(new_snapshot.data.as_slice()).map_err(|e| {
                StorageError::read_snapshot(Some(new_snapshot.meta.signature()), &e)
            })?;
            let application_data = A::import(snapshot).map_err(|e| {
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

    async fn get_current_snapshot(&mut self) -> Result<Option<Snapshot<A::C>>, io::Error> {
        match &*self.current_snapshot.read().await {
            Some(snapshot) => Ok(Some(Snapshot {
                meta: snapshot.meta.clone(),
                snapshot: snapshot.data.clone(),
            })),
            None => Ok(None),
        }
    }
}
