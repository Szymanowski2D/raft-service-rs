use openraft::RaftNetworkFactory;

use crate::raft::config::type_config::Node;
use crate::raft::config::type_config::NodeId;
use crate::raft::config::type_config::TypeConfig;
use crate::raft::network::Network;
use crate::raft::network::v2::NetworkConnection;

impl RaftNetworkFactory<TypeConfig> for Network {
    type Network = NetworkConnection;

    async fn new_client(&mut self, _target: NodeId, node: &Node) -> Self::Network {
        NetworkConnection::new(node.clone())
    }
}
