#!/usr/bin/env bash

echo "starting ethereum-service mock"

kafkacat -C -b kafka:9092 -t request -f '{"status": "added", "txid": "%T", "message": "%s"}\n' -u \
    | grep --line-buffered -v "%.*" \
    | { while read -r message; do
            sleep 0.$[ ( $RANDOM % 100 ) ]s
            sleep 0.$[ ( $RANDOM % 100 ) ]s
            echo $message
        done } \
    | kafkacat -P -b kafka:9092 -t response