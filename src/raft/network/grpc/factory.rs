use openraft::RaftNetworkFactory;

use crate::raft::config::type_config::Node;
use crate::raft::config::type_config::NodeId;
use crate::raft::config::type_config::TypeConfig;
use crate::raft::network::grpc::GRPCNetwork;
use crate::raft::network::grpc::v2::GRPCNetworkConnection;

impl RaftNetworkFactory<TypeConfig> for GRPCNetwork {
    type Network = GRPCNetworkConnection;

    async fn new_client(&mut self, _target: NodeId, node: &Node) -> Self::Network {
        GRPCNetworkConnection::new(node.clone())
    }
}
