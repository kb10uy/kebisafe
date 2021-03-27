# Build backend
FROM rust:1.50 AS builder
WORKDIR /build
COPY . .
RUN cargo install --path .

# Build frontend
FROM node:15-alpine3.13 AS bundler
WORKDIR /build
COPY . .
RUN apk add --no-cache yarn
RUN yarn && yarn build

# Merge them
FROM debian:bullseye-slim
LABEL maintainer="kb10uy"
COPY --from=builder /usr/local/cargo/bin/kebisafe /usr/local/bin/kebisafe
COPY --from=bundler /build/public /public
RUN mkdir /media

WORKDIR /
CMD ["kebisafe", "serve"]
