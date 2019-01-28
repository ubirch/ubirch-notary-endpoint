#!/usr/bin/env bash

if ! which docker-compose; then
    echo "docker-compose is required to run this script"
    exit 1
fi

docker-compose up -d

kcat="docker run --network=container:kafka --rm -it ryane/kafkacat"

while ! $kcat -L -b kafka:9092 -t request > /dev/null; do
    echo "trying again..."
done

echo "running kafkacat for real"

read -r -d '' PROG <<EOF
    kafkacat -C -b kafka:9092 -t request -f '{"status": "added", "txid": "%T", "message": "%s"}\n' -u | grep --line-buffered -v "%.*" | \
        { while read -r message; do
            sleep 0.\$[ ( \$RANDOM % 100 ) ]s
            sleep 0.\$[ ( \$RANDOM % 100 ) ]s
            echo \$message
        done } | kafkacat -P -b kafka:9092 -t response
EOF

docker run --network=container:kafka --rm -it --entrypoint "/bin/bash" ryane/kafkacat -c "$PROG"