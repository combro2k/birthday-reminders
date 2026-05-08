# syntax=docker/dockerfile:1.7

# Build stage
FROM rust:1.95-alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconf ca-certificates

WORKDIR /app

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release --locked && rm -rf src target/release/birthday-reminders*

# Build application
COPY src/ src/
COPY migrations/ migrations/
COPY static/ static/
COPY templates/ templates/
RUN touch src/main.rs && cargo build --release --locked

# Prepare minimal runtime filesystem with non-root identity
FROM alpine:3.20 AS runtime-files

RUN addgroup -S -g 10001 app && adduser -S -D -H -u 10001 -G app app
RUN mkdir -p /app/etc /app/data && chown -R 10001:10001 /app

COPY config.yaml.example /app/etc/config.yaml.example

# Runtime stage
FROM scratch

WORKDIR /app

# Needed for outbound TLS (OIDC, SMTP, webhook providers)
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt

# Preserve user/group lookup for the numeric runtime UID/GID
COPY --from=runtime-files /etc/passwd /etc/passwd
COPY --from=runtime-files /etc/group /etc/group

COPY --from=builder /app/target/release/birthday-reminders /app/bin/birthday-reminders
COPY --from=runtime-files /app/etc/config.yaml.example /app/etc/config.yaml.example
COPY --from=runtime-files /app/data /app/data

USER 10001:10001

EXPOSE 3000

ENTRYPOINT ["/app/bin/birthday-reminders"]
CMD ["-c", "/app/etc/config.yaml", "serve"]
