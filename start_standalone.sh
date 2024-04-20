#!/bin/sh

set -o errexit

cargo build

kill_all() {
    SERVICE='raft-key-value-rocks'
    if [ "$(uname)" = "Darwin" ]; then
        if pgrep -xq -- "${SERVICE}"; then
            pkill -f "${SERVICE}"
        fi
        rm -r *-db || echo "no db to clean"
    else
        set +e # killall will error if finds no process to kill
        killall "${SERVICE}"
        set -e
    fi
}

rpc() {
    local uri=$1
    local body="$2"

    echo '---'" rpc(:$uri, $body)"

    {
        if [ ".$body" = "." ]; then
            time curl --silent "127.0.0.1:$uri"
        else
            time curl --silent "127.0.0.1:$uri" -H "Content-Type: application/json" -d "$body"
        fi
    } | {
        if type jq > /dev/null 2>&1; then
            jq
        else
            cat
        fi
    }

    echo
    echo
}

export RUST_LOG=trace
export RUST_BACKTRACE=full
bin=./target/debug/s3-server

echo "Killing all running raft-key-value-rocks and cleaning up old data"

kill_all
sleep 1

if ls *-db
then
    rm -r *-db || echo "no db to clean"
fi

${bin} --id 1 --http-addr 127.0.0.1:21001 --rpc-addr 127.0.0.1:22001 2>&1 > n1.log &
PID1=$!
sleep 1
echo "Server started"

echo "Initialize server as a single-node cluster"
sleep 2
echo
rpc 21001/cluster/init '{}'