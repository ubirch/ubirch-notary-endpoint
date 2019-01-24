FROM rust:1.32.0

WORKDIR /usr/src/ubirch-notary-endpoint
COPY . .

RUN cargo install --path .

CMD ["ubirch-notary-endpoint"]