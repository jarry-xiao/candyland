FROM solanalabs/solana:v1.10.10 as builder
RUN apt-get update \
      && apt-get -y install \
           wget \
           curl \
           build-essential \
           software-properties-common \
           lsb-release \
           libelf-dev \
           linux-headers-generic \
           pkg-config
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
WORKDIR /rust/
COPY deps /rust/deps
WORKDIR /rust/deps/metaplex-program-library/token-metadata/program
RUN cargo build-bpf --bpf-out-dir /so/
WORKDIR /rust/deps/solana-program-library/associated-token-account/program
RUN cargo build-bpf --bpf-out-dir /so/
WORKDIR /rust/deps/solana-program-library/token/program-2022
RUN cargo build-bpf --bpf-out-dir /so/
WORKDIR /rust/deps/solana-program-library/token/program
RUN cargo build-bpf --bpf-out-dir /so/
COPY lib /rust/lib
COPY plerkle_serialization /rust/plerkle_serialization
COPY digital_asset_types /rust/digital_asset_types
COPY messenger /rust/messenger
COPY contracts /rust/contracts
WORKDIR /rust/contracts
RUN cargo build-bpf --bpf-out-dir /so/
COPY plerkle /rust/plerkle
WORKDIR /rust/plerkle
RUN cargo build

FROM solanalabs/solana:v1.10.10
COPY --from=builder /rust/plerkle/target/debug/libplerkle.so /plugin/plugin.so
COPY --from=builder /so/ /so/
RUN apt-get update; apt-get -y install curl
COPY ./docker .
RUN chmod +x ./*.sh
ENTRYPOINT [ "./runs.sh" ]
CMD [""]
