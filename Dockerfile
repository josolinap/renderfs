############################################################################################
####  SERVER
############################################################################################

# Use the official Rust image instead of musl to avoid OpenSSL issues
FROM rust:1.75 AS chef
USER root
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY ./pentaract .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY ./pentaract .
RUN cargo build --release

############################################################################################
####  UI
############################################################################################

FROM node:21-slim AS ui
WORKDIR /app
COPY ./ui .
RUN npm install -g pnpm
RUN pnpm i
ENV VITE_API_BASE /api
RUN pnpm run build

############################################################################################
####  RUNNING
############################################################################################

# Use alpine as base for runtime to have proper SSL certs
FROM alpine:latest AS runtime
RUN apk --no-cache add ca-certificates
COPY --from=builder /app/target/release/pentaract /pentaract
COPY --from=ui /app/dist /ui
EXPOSE 8000
ENTRYPOINT ["/pentaract"]
