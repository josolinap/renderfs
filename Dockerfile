############################################################################################
####  SERVER
############################################################################################

# Use the official Rust image
FROM rust:1.75 AS builder
WORKDIR /app

# Copy source code
COPY ./pentaract .

# Build the application
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
# Create directory and copy binary with proper permissions
RUN mkdir -p /app
COPY --from=builder /app/target/release/pentaract /app/pentaract
COPY --from=ui /app/dist ./ui
WORKDIR /app
EXPOSE 8000
# Make binary executable and run it with absolute path
RUN chmod +x /app/pentaract
ENTRYPOINT ["/app/pentaract"]
