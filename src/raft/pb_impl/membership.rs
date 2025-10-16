use std::collections::BTreeMap;

use crate::pb::Membership as PbMembership;
use crate::pb::NodeIdSet;
use crate::raft::config::type_config::Membership;

impl From<PbMembership> for Membership {
    fn from(membership: PbMembership) -> Self {
        let mut configs = vec![];
        for c in membership.configs {
            let config = c.node_ids.into_keys().collect();
            configs.push(config);
        }

        Membership::new(configs, membership.nodes).unwrap()
    }
}

impl From<Membership> for PbMembership {
    fn from(membership: Membership) -> Self {
        let mut configs = vec![];
        for c in membership.get_joint_config() {
            let mut node_ids = BTreeMap::new();
            for nid in c.iter() {
                node_ids.insert(*nid, ());
            }
            configs.push(NodeIdSet { node_ids });
        }

        let nodes = membership
            .nodes()
            .map(|(nid, n)| (*nid, n.clone()))
            .collect();

        PbMembership { configs, nodes }
    }
}
