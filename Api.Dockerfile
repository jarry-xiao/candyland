FROM rust:1.60-bullseye as builder
ARG MODE=debug
RUN apt-get update -y && \
    apt-get install -y build-essential make git
WORKDIR /rust
RUN USER=root cargo new --lib nft_api
COPY contracts /rust/contracts
COPY lib /rust/lib
COPY deps /rust/deps
COPY plerkle /rust/plerkle
COPY plerkle_serialization /rust/plerkle_serialization
COPY digital_asset_types /rust/digital_asset_types
COPY messenger /rust/messenger
WORKDIR /rust/nft_api
COPY ./nft_api/Cargo.toml ./Cargo.toml

RUN cargo build

COPY ./nft_api .
RUN cargo build
RUN cp -r /rust/nft_api/target/$MODE /rust/bin
RUN rm -rf /rust/nft_api/target/

FROM rust:1.61-slim-bullseye
ARG APP=/usr/src/app
RUN apt update \
    && apt install -y curl ca-certificates tzdata \
    && rm -rf /var/lib/apt/lists/*
ENV TZ=Etc/UTC \
    APP_USER=appuser
RUN groupadd $APP_USER \
    && useradd -g $APP_USER $APP_USER \
    && mkdir -p ${APP}
COPY --from=builder /rust/bin/nft_api ${APP}
RUN chown -R $APP_USER:$APP_USER ${APP}
USER $APP_USER
WORKDIR ${APP}
CMD /usr/src/app/nft_api
