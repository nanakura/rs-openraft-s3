# rs-s3-local

#### Quick Start
```shell
cargo run -- --id 1 --http-addr "127.0.0.1:31001" --rpc-addr "127.0.0.1:32001"
```
init cluster
```shell
curl "127.0.0.1:31001/cluster/init" -H "Content-Type: application/json" -d "{}"
```