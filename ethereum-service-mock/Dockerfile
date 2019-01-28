FROM ryane/kafkacat

COPY ethereum-service-mock.sh .

# this entrypoint will restart ethereum-service-mock.sh if it exits
ENTRYPOINT [ "bash", "-c", "while true; do ./ethereum-service-mock.sh; sleep 1s; done" ]