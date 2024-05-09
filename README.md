# rs-s3-local

#### Install
```shell
git clone https://github.com/nanakura/rs-openraft-s3
cd rs-openraft-s3
cargo install --path .
s3-server --help
```
#### Usage
```shell
Usage: s3-server.exe [OPTIONS]

Options:
      --id <ID>                              [default: 1]
      --http-addr <HTTP_ADDR>                [default: 127.0.0.1:9000]
      --rpc-addr <RPC_ADDR>                  [default: 127.0.0.1:32001]
      --leader-http-addr <LEADER_HTTP_ADDR>
      --access-key <ACCESS_KEY>              [default: minioadmin]
      --secret-key <SECRET_KEY>              [default: minioadmin]
  -h, --help                                 Print help
  -V, --version                              Print version

```

