# Raft

## Test

```bash
RUST_BACKTRACE=1 RUST_LOG=trace,openraft=off,h2=off,tonic=off,server_handshake=off,tower=off,hyper_util=off cargo test test_cluster
```