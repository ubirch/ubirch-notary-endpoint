FROM rust:1.32.0 as build

# create a new empty shell project
RUN USER=root cargo new --bin ubirch-notary-endpoint
WORKDIR /ubirch-notary-endpoint

# copy over your manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# this build step will cache your dependencies
RUN cargo build --release
RUN rm src/*.rs

# copy your source tree
COPY ./src ./src

# build for release
RUN rm ./target/release/deps/*ubirch_notary_endpoint*

# actual project build, the dependencies have been cached
RUN cargo build --release

# our final base
FROM rust:1.32.0-slim

# copy the build artifact from the build stage
COPY --from=build /ubirch-notary-endpoint/target/release/ubirch-notary-endpoint .

# set the startup command to run your binary
CMD ["./ubirch-notary-endpoint"]