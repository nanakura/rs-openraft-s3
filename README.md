# rs-openraft-s3
An experimental generic S3 server

### Install
```shell
git clone https://github.com/nanakura/rs-openraft-s3
cd rs-openraft-s3
cargo install --path .
s3-server --help
```
### Usage
#### Standalone
```shell
Usage: s3-server.exe [OPTIONS]

Options:
      --id <ID>                              [default: 1]
      --http-addr <HTTP_ADDR>                [default: 127.0.0.1:9000]
      --rpc-addr <RPC_ADDR>                  [default: 127.0.0.1:32001]
      --fs-root <FS_ROOT>                    [default: .]
      --leader-http-addr <LEADER_HTTP_ADDR>
      --access-key <ACCESS_KEY>              [default: minioadmin]
      --secret-key <SECRET_KEY>              [default: minioadmin]
  -h, --help                                 Print help
  -V, --version                              Print version

```

#### Cluster

master node

```shell
s3-server --id 1 --http-addr "127.0.0.1:9000" --rpc-addr "127.0.0.1:32000"
```

other nodes

```shell
s3-server --id 2 --http-addr "127.0.0.1:9001" --rpc-addr "127.0.0.1:32001" --leader-http-addr 127.0.0.1:9000
s3-server --id 3 --http-addr "127.0.0.1:9002" --rpc-addr "127.0.0.1:32002" --leader-http-addr 127.0.0.1:9000
```

