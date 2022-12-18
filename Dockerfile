FROM rust:1.66.0 as builder
WORKDIR /usr/src/indexer
COPY . .
RUN cargo install --path .

FROM debian:bullseye-slim
RUN apt-get update && apt-get install openssl && apt-get install ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/indexer /usr/local/bin/indexer
COPY --from=builder /usr/src/indexer/.env .env
CMD ["indexer"]