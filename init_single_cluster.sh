#!/bin/bash

set -e

if [ $# -ne 1 ]; then
  echo "Usage: $0 <rpc_addr>"
  echo "Example: $0 127.0.0.1:5001"
  exit 1
fi

GRPC_SERVER="$1"
SERVICE_METHOD="controller.RaftControllerService.Init"
INIT_REQUEST_JSON=$(cat <<EOF
{
  "nodes": [
    {
      "node_id": 0,
      "rpc_addr": "${GRPC_SERVER}"
    }
  ]
}
EOF
)

grpcurl -plaintext -import-path ./proto -proto controller.proto -d "${INIT_REQUEST_JSON}" "${GRPC_SERVER}" "${SERVICE_METHOD}"

echo "✅ Raft single-node cluster initialized successfully!"
