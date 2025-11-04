use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::ApplicationConfig;
use crate::ApplicationData;
use crate::raft::config::type_config::Snapshot;
use crate::raft::config::type_config::SnapshotData;
use crate::raft::config::type_config::SnapshotMeta;
use crate::raft::config::type_config::StorageError;
use crate::raft::config::type_config::TypeConfig;
use crate::raft::state_machine::store::StateMachineStore;
use openraft::RaftSnapshotBuilder;
use prost::Message;

pub(super) struct StoredSnapshot {
    pub(super) meta: SnapshotMeta,
    pub(super) data: SnapshotData,
}

impl<C: ApplicationConfig> RaftSnapshotBuilder<TypeConfig> for Arc<StateMachineStore<C>> {
    async fn build_snapshot(&mut self) -> Result<Snapshot, StorageError> {
        let state_machine = self.state_machine.read().await;

        let data = state_machine.application_data.export().encode_to_vec();
        let last_applied_log = state_machine.last_applied_log;
        let last_membership = state_machine.last_membership.clone();

        // Lock the current snapshot before releasing the lock on the state machine, to avoid a race
        // condition on the written snapshot
        let mut current_snapshot = self.current_snapshot.write().await;
        drop(state_machine);

        let snapshot_idx = self.snapshot_idx.fetch_add(1, Ordering::Relaxed) + 1;
        let snapshot_id = if let Some(last) = last_applied_log {
            format!(
                "{}-{}-{}",
                last.committed_leader_id(),
                last.index(),
                snapshot_idx
            )
        } else {
            format!("--{snapshot_idx}")
        };

        let meta = SnapshotMeta {
            last_log_id: last_applied_log,
            last_membership,
            snapshot_id,
        };

        let snapshot = StoredSnapshot {
            meta: meta.clone(),
            data: data.clone(),
        };

        *current_snapshot = Some(snapshot);

        Ok(Snapshot {
            meta,
            snapshot: data,
        })
    }
}
