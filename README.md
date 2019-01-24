# Ubirch Notary Endpoint

Simple endpoint for use with `ubirch-ethereum-service` (and possibly others).

The application is written in Rust, using `warp` and `rdkafka`.

## Building
You can build the executable using Rust, or just go straight for building the
docker image - no need to set the rust toolchain up in that case.

### Rust
```bash
cargo build --release
``` 

### Docker
```bash
docker build -t ubirch/notary-endpoint .
```

## Configuration
The application is configured with the following environment variables:

| variable name | purpose |
|---------------|---------|
| KAFKA_BROKERS | comma separated list of kafka brokers: `(<broker host>:<broker port>),*` |
| REQUEST_TOPIC | topic to send requests to for the ethereum service |
| RESPONSE_TOPIC | topic that ethereum service uses to respond |
| ERROR_TOPIC | topic that ethereum service uses to provide errors |
| EXPLORER_URL | url of the blockchain explorer; `/<txid>` will be appended to it when responding to user |
| BIND_ADDR | address that this server listens on `<ip>:<port>` |

## Endpoints

* `POST /` - submit `body` to anchor in ethereum; responses:
    * `{"Ok": {"explorer_url": "<EXPLORER_URL>/<txid>"}}` - contains link to blockchain explorer page containing 
    transaction info related to submitted `body`
    * `{"CheckAgain": {"url": "/check/<hex>"}}` - contains address of where to check again if the anchoring process 
    was completed
    * `{"Error": {"message": "<error-message>"}}` - contains error message
* `GET /check/<hex>` - checks if the anchoring process has been completed; responses: see above

## TODO
* actually handle messages from `ERROR_TOPIC`
* clear the cache somehow - currently it's unbounded
