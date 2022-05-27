FROM rust:1.60-bullseye as builder
ARG MODE=debug
RUN apt-get update -y && \
    apt-get install -y build-essential make git
WORKDIR /rust
RUN USER=root cargo new --lib nft_ingester
COPY contracts /rust/contracts
COPY plerkle /rust/plerkle
COPY deps /rust/deps
COPY plerkle_serialization /rust/plerkle_serialization
COPY messenger /rust/messenger
WORKDIR /rust/nft_ingester
COPY ./nft_ingester/Cargo.toml ./Cargo.toml

RUN cargo build

COPY ./nft_ingester .
RUN ls -la
RUN cargo build
RUN cp -r /rust/nft_ingester/target/$MODE /rust/bin

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
COPY --from=builder /rust/bin ${APP}
RUN chown -R $APP_USER:$APP_USER ${APP}
USER $APP_USER
WORKDIR ${APP}
CMD /usr/src/app/nft_ingester
