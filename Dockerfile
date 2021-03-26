FROM rust:1.50 AS builder

WORKDIR /build
COPY . .
RUN cargo install --path .

FROM debian:bullseye-slim
COPY --from=builder /usr/local/cargo/bin/kebisafe /usr/local/bin/kebisafe
CMD ["kebisafe", "serve"]
