use std::fmt::Debug;

use openraft::type_config::alias::*;

use crate::pb;

openraft::declare_raft_types!(
    pub TypeConfig:
        D                = Vec<u8>,
        R                = bool,
        Node             = pb::Node,
        LeaderId         = pb::LeaderId,
        Vote             = pb::Vote,
        Entry            = pb::Entry,
        SnapshotData     = Vec<u8>,
);

pub(crate) type Raft = openraft::Raft<TypeConfig>;
pub(crate) type LogId = openraft::LogId<TypeConfig>;
pub(crate) type StoredMembership = openraft::StoredMembership<TypeConfig>;
pub(crate) type Snapshot = openraft::Snapshot<TypeConfig>;
pub(crate) type StorageError = openraft::StorageError<TypeConfig>;
pub(crate) type SnapshotMeta = openraft::SnapshotMeta<TypeConfig>;
pub(crate) type Membership = openraft::Membership<TypeConfig>;

pub(crate) type SnapshotData = SnapshotDataOf<TypeConfig>;
pub(crate) type Entry = EntryOf<TypeConfig>;
pub(crate) type Term = TermOf<TypeConfig>;
pub(crate) type NodeId = NodeIdOf<TypeConfig>;
pub(crate) type Node = NodeOf<TypeConfig>;
pub(crate) type Vote = VoteOf<TypeConfig>;
pub(crate) type LeaderId = LeaderIdOf<TypeConfig>;
pub(crate) type CommittedLeaderId = CommittedLeaderIdOf<TypeConfig>;

pub(crate) type VoteRequest = openraft::raft::VoteRequest<TypeConfig>;
pub(crate) type VoteResponse = openraft::raft::VoteResponse<TypeConfig>;
pub(crate) type AppendEntriesRequest = openraft::raft::AppendEntriesRequest<TypeConfig>;
pub(crate) type AppendEntriesResponse = openraft::raft::AppendEntriesResponse<TypeConfig>;
pub(crate) type SnapshotResponse = openraft::raft::SnapshotResponse<TypeConfig>;
pub(crate) type ClientWriteResponse = openraft::raft::ClientWriteResponse<TypeConfig>;

pub(crate) type RaftError<E> = openraft::error::RaftError<TypeConfig, E>;
pub(crate) type RPCError = openraft::error::RPCError<TypeConfig>;
pub(crate) type StreamingError = openraft::error::StreamingError<TypeConfig>;
pub(crate) type ClientWriteError = openraft::error::ClientWriteError<TypeConfig>;
pub(crate) type CheckIsLeaderError = openraft::error::CheckIsLeaderError<TypeConfig>;

pub(crate) type EntryPayload = openraft::entry::EntryPayload<TypeConfig>;
