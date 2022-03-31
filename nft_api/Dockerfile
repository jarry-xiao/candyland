FROM rust:1.59 as builder
RUN USER=root cargo new --bin nft_api
WORKDIR ./nft_api
COPY ./Cargo.toml ./Cargo.toml
RUN apt-get update -y && \
    apt-get install -y build-essential make git libgflags-dev libsnappy-dev cmake libclang-dev libtbb-dev zlib1g-dev libbz2-dev libjemalloc-dev
RUN rustup component add  rustfmt && \
    rustup toolchain install nightly --component rustfmt --component clippy --allow-downgrade && \
    rustup default nightly
RUN cargo build --release
RUN rm src/*.rs
ADD . ./
RUN rm ./target/release/deps/nft_api*
RUN cargo build --release

FROM rust:1.59-slim
ARG APP=/usr/src/app
RUN apt update \
    && apt install -y ca-certificates tzdata \
    && rm -rf /var/lib/apt/lists/*

ENV TZ=Etc/UTC \
    APP_USER=appuser

RUN groupadd $APP_USER \
    && useradd -g $APP_USER $APP_USER \
    && mkdir -p ${APP}

COPY --from=builder /nft_api/target/release/nft_api ${APP}/nft_api

RUN chown -R $APP_USER:$APP_USER ${APP}

USER $APP_USER
WORKDIR ${APP}

CMD ["./nft_api"]