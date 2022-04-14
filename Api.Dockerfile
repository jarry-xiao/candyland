#FROM rust:1.59 as builder
#RUN USER=root cargo new --lib nft_api
#COPY ./programs programs
#WORKDIR ./nft_api
#COPY ./nft_api/CacheCargo.toml ./Cargo.toml
#RUN apt-get update -y && \
#    apt-get install -y build-essential make git
#RUN rustup component add  rustfmt && \
#    rustup toolchain install nightly --component rustfmt --component clippy --allow-downgrade && \
#    rustup default nightly
#RUN cargo build --release
#RUN rm src/*.rs
#COPY nft_api .
#RUN rm ./target/release/deps/nft_api*
#RUN cargo build --release

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

#COPY --from=builder /nft_api/target/release ${APP}

RUN chown -R $APP_USER:$APP_USER ${APP}

USER $APP_USER
WORKDIR ${APP}
ENV BIN=/bin/api
CMD ${BIN}