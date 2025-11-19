use openraft::RaftNetworkFactory;

use crate::application::ApplicationConfig;
use crate::raft::config::type_config::Node;
use crate::raft::config::type_config::NodeId;
use crate::raft::config::type_config::TypeConfig;
use crate::raft::network::grpc::GRPCNetwork;
use crate::raft::network::grpc::v2::GRPCNetworkConnection;

impl<C> RaftNetworkFactory<TypeConfig<C>> for GRPCNetwork
where
    C: ApplicationConfig,
{
    type Network = GRPCNetworkConnection<C>;

    async fn new_client(&mut self, _target: NodeId<C>, node: &Node<C>) -> Self::Network {
        GRPCNetworkConnection::new(node.clone())
    }
}
