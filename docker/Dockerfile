FROM solanalabs/solana:v1.10.10
RUN apt-get update; apt-get -y install curl
COPY . .
RUN chmod +x ./*.sh
ENTRYPOINT [ "./runs.sh" ]
CMD [""]

