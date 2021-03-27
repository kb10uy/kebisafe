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
RUN mkdir -p /var/www/kebisafe/media
COPY --from=builder /usr/local/cargo/bin/kebisafe /var/www/kebisafe/kebisafe
COPY --from=bundler /build/public /var/www/kebisafe/public

EXPOSE 9375
WORKDIR /var/www/kebisafe
CMD ["/var/www/kebisafe/kebisafe", "serve"]
