# Build stage
FROM rust:1.85-alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconf

WORKDIR /app

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src target/release/birthday-reminders*

# Build application
COPY src/ src/
COPY migrations/ migrations/
COPY static/ static/
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM alpine:3.20

RUN apk add --no-cache ca-certificates

RUN addgroup -S app && adduser -S -G app app

WORKDIR /app

COPY --from=builder /app/target/release/birthday-reminders /app/bin/birthday-reminders
COPY config.yaml.example /app/etc/config.yaml.example

RUN mkdir -p /app/data && chown -R app:app /app

USER app

EXPOSE 3000

ENTRYPOINT ["/app/bin/birthday-reminders"]
CMD ["-c", "/app/etc/config.yaml", "serve"]
