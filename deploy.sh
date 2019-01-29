#!/usr/bin/env bash
docker build -t ubirch/ubirch-notary-endpoint:latest .
docker push ubirch/ubirch-notary-endpoint:latest