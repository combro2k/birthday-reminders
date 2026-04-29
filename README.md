# Birthday Reminders

A self-hosted birthday reminder application with a web UI, CLI, and flexible notification channels. Built with Rust, Axum, and SQLx.

## Features

- **Web UI** — manage birthdays, notification channels, and user settings
- **CLI** — add/list/remove birthdays and manage users from the terminal
- **Notifications** — Email, Gotify, Telegram, Signal, and WhatsApp
- **Scheduled reminders** — configurable cron schedule with customizable lead days
- **Authentication** — local accounts with Argon2 password hashing, or OIDC (Keycloak, Authentik, Zitadel, and others)
- **Encryption** — notification channel secrets encrypted at rest using XChaCha20-Poly1305
- **API tokens** — programmatic access for automation
- **PWA support** — installable as a progressive web app

## Prerequisites

- **Rust** 1.85+ (edition 2024)
- **PostgreSQL** 13+ or **SQLite** 3.35+
- A running database instance (for PostgreSQL)

## Quick Start

### 1. Build

```bash
cargo build --release
```

### 2. Configure

```bash
cp config.yaml.example config.yaml
# Edit config.yaml with your settings
```

### 3. Create a user

```bash
./target/release/birthday-reminders -c config.yaml create-user \
  --username admin \
  --email admin@example.com \
  --password changeme \
  --admin
```

### 4. Start the server

```bash
./target/release/birthday-reminders -c config.yaml serve
```

The app will be available at `http://localhost:3000` by default.

## Configuration

Configuration is stored in a YAML file (default: `config.yaml`). See [`config.yaml.example`](config.yaml.example) for a fully commented example.

### Database

Supports SQLite and PostgreSQL:

```yaml
database:
  # SQLite (default, simplest for single-user/small deployments)
  url: "sqlite://birthday_reminders.db"
  max_connections: 10
```

```yaml
database:
  # PostgreSQL (recommended for multi-user or production deployments)
  url: "postgres://birthday:birthday@localhost:5432/birthday_reminders"
  max_connections: 10
```

Database migrations run automatically on startup.

### Migrating from SQLite to PostgreSQL

If you started with SQLite and want to move to PostgreSQL:

1. **Export data from SQLite:**

   ```bash
   sqlite3 birthday_reminders.db .dump > backup.sql
   ```

2. **Create the PostgreSQL database:**

   ```bash
   createdb birthday_reminders
   ```

3. **Start the app once with the new PostgreSQL URL** to run migrations:

   ```bash
   birthday-reminders -c config.yaml serve
   # Stop after it starts successfully (Ctrl+C)
   ```

4. **Import your data.** SQLite's dump format isn't directly compatible with PostgreSQL. Use a tool like [pgloader](https://pgloader.io/) for automatic conversion:

   ```bash
   pgloader sqlite:///path/to/birthday_reminders.db \
            postgresql://birthday:birthday@localhost/birthday_reminders
   ```

   Or manually export and transform the data using CSV:

   ```bash
   # Export each table from SQLite
   sqlite3 -header -csv birthday_reminders.db "SELECT * FROM users;" > users.csv
   sqlite3 -header -csv birthday_reminders.db "SELECT * FROM birthdays;" > birthdays.csv
   sqlite3 -header -csv birthday_reminders.db "SELECT * FROM notification_channels;" > channels.csv

   # Import into PostgreSQL (after migrations have run)
   psql birthday_reminders -c "\copy users FROM 'users.csv' CSV HEADER"
   psql birthday_reminders -c "\copy birthdays FROM 'birthdays.csv' CSV HEADER"
   psql birthday_reminders -c "\copy notification_channels FROM 'channels.csv' CSV HEADER"
   ```

5. **Update `config.yaml`** to use the PostgreSQL URL and restart.

### Server

```yaml
server:
  listen: "0.0.0.0:3000"
  base_url: "http://localhost:3000"
  session_secret: "generate-a-random-64-char-string-here"
  encryption_key: "generate-a-separate-key-for-encryption"  # optional, derives from session_secret if omitted
  static_dir: "static"  # path to static assets (CSS, JS, manifest)
```

> **Important:** Generate a strong random `session_secret` for production. It must be at least 32 characters.
>
> The `encryption_key` is used to encrypt notification channel secrets at rest (XChaCha20-Poly1305). If not set, it defaults to the `session_secret`. For best security, set a dedicated key with: `openssl rand -base64 32`

### Authentication

```yaml
auth:
  # Allow new users to self-register via the web UI (default: false)
  allow_registration: false
```

When `allow_registration` is `true`, a "Register" link appears on the login page allowing anyone to create an account. When `false` (default), only administrators can create new users via the admin panel or CLI.

### Reminders

```yaml
reminders:
  # Cron expression: sec min hour day month weekday
  schedule: "0 0 8 * * *"       # Every day at 08:00
  default_days_before: [7, 3, 1, 0]  # Remind 7, 3, and 1 day(s) before + on the day
```

The `default_days_before` setting is the global default. Each user can override this from **Settings → Profile → Reminder Preferences** in the web UI, choosing from 14, 7, 3, or 1 day(s) before and/or on the day itself.

### Logging

```yaml
logging:
  # "stdout" (default) or "syslog"
  output: "stdout"
  # Log level filter (supports RUST_LOG syntax)
  level: "info"
```

Set `output: "syslog"` to send logs to the system's syslog daemon (via Unix socket). This is useful for systemd/journald or traditional syslog setups.

The `level` field accepts [RUST_LOG-style](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html) directives, e.g. `"birthday_reminders=debug,tower_http=info"`. The `RUST_LOG` environment variable takes precedence if set.

## OIDC Authentication

Birthday Reminders supports OpenID Connect for single sign-on. When configured, a "Sign in with {provider_name}" button appears on the login page.

### OIDC Configuration Options

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `enabled` | No | `false` | Enable OIDC authentication |
| `provider_name` | Yes | — | Display name shown on the login button |
| `issuer_url` | Yes | — | OIDC discovery URL for your provider |
| `client_id` | Yes | — | OAuth2 client ID |
| `client_secret` | Yes | — | OAuth2 client secret |
| `scopes` | No | `["openid", "profile", "email"]` | Requested OIDC scopes |
| `auto_provision` | No | `true` | Create local accounts on first OIDC login |
| `default_role` | No | `"user"` | Role assigned to auto-provisioned users |

The callback URL to configure in your provider is:

```
{base_url}/auth/oidc/callback
```

For example: `http://localhost:3000/auth/oidc/callback`

---

### Keycloak

1. Create a new client in your realm:
   - **Client ID:** `birthday-reminders`
   - **Client Protocol:** `openid-connect`
   - **Access Type:** `confidential`
2. Set **Valid Redirect URIs** to `https://your-domain.com/auth/oidc/callback`
3. Copy the client secret from the **Credentials** tab

```yaml
auth:
  oidc:
    enabled: true
    provider_name: "Keycloak"
    issuer_url: "https://keycloak.example.com/realms/myrealm"
    client_id: "birthday-reminders"
    client_secret: "your-client-secret"
    scopes: ["openid", "profile", "email"]
    auto_provision: true
    default_role: "user"
```

---

### Authentik

1. In Authentik, go to **Applications → Providers** and create an **OAuth2/OpenID Provider**:
   - **Name:** `birthday-reminders`
   - **Authorization flow:** choose your preferred flow
   - **Redirect URIs:** `https://your-domain.com/auth/oidc/callback`
2. Create an **Application** and link it to the provider
3. The issuer URL follows the pattern: `https://authentik.example.com/application/o/<application-slug>/`

```yaml
auth:
  oidc:
    enabled: true
    provider_name: "Authentik"
    issuer_url: "https://authentik.example.com/application/o/birthday-reminders/"
    client_id: "birthday-reminders"
    client_secret: "your-client-secret"
```

---

### Zitadel

1. In Zitadel, create a new **Project** and add an **Application** of type **Web**:
   - **Authentication Method:** `POST` (for client secret)
   - **Redirect URIs:** `https://your-domain.com/auth/oidc/callback`
2. The client ID format in Zitadel is typically `<numeric-id>@<project-name>`
3. The issuer URL is your Zitadel instance root URL

```yaml
auth:
  oidc:
    enabled: true
    provider_name: "Zitadel"
    issuer_url: "https://zitadel.example.com"
    client_id: "123456@birthday-reminders"
    client_secret: "your-client-secret"
```

---

### Authelia

1. Add an OpenID Connect client in your Authelia configuration:
   ```yaml
   identity_providers:
     oidc:
       clients:
         - client_id: birthday-reminders
           client_secret: '$pbkdf2-sha512$...'  # hashed secret
           redirect_uris:
             - https://your-domain.com/auth/oidc/callback
           scopes:
             - openid
             - profile
             - email
   ```

2. Configure Birthday Reminders:

```yaml
auth:
  oidc:
    enabled: true
    provider_name: "Authelia"
    issuer_url: "https://auth.example.com"
    client_id: "birthday-reminders"
    client_secret: "your-plain-client-secret"
```

---

### Generic OIDC Provider

Any provider that supports OpenID Connect Discovery (i.e., exposes `/.well-known/openid-configuration`) should work:

```yaml
auth:
  oidc:
    enabled: true
    provider_name: "My Provider"
    issuer_url: "https://idp.example.com"
    client_id: "your-client-id"
    client_secret: "your-client-secret"
```

## CLI Usage

```
birthday-reminders [OPTIONS] <COMMAND>

Commands:
  serve            Start the web server
  create-user      Create a new user (direct DB, no server needed)
  add              Add a birthday (requires --token)
  list             List all birthdays (requires --token)
  upcoming         Show upcoming birthdays (requires --token)
  remove           Remove a birthday by ID (requires --token)
  check-reminders  Manually trigger reminder check for all users

Options:
  -c, --config <PATH>  Path to config file [default: config.yaml]
```

### Examples

```bash
# Add a birthday
birthday-reminders add "Jane Doe" 1990-05-15 --token your-api-token

# List upcoming birthdays in the next 14 days
birthday-reminders upcoming --days 14 --token your-api-token

# Manually trigger reminders (useful for testing)
birthday-reminders check-reminders
```

## Notification Channels

Users can configure one or more notification channels in the web UI under **Settings → Notification Channels**:

| Channel | Description |
|---------|-------------|
| **Email** | SMTP-based email notifications |
| **Gotify** | Push notifications via a Gotify server |
| **Telegram** | Messages via Telegram Bot API |
| **Signal** | Messages via Signal messenger |
| **WhatsApp** | Messages via WhatsApp Business API |

## Installation

### Docker

```bash
# Build the image
docker build -t birthday-reminders .

# Run with SQLite (simplest)
docker run -d \
  --name birthday-reminders \
  -p 3000:3000 \
  -v birthday-data:/app/data \
  -v ./config.yaml:/app/etc/config.yaml:ro \
  birthday-reminders

# Run with PostgreSQL
docker run -d \
  --name birthday-reminders \
  -p 3000:3000 \
  -v ./config.yaml:/app/etc/config.yaml:ro \
  birthday-reminders
```

Or with Docker Compose:

```yaml
services:
  app:
    build: .
    ports:
      - "3000:3000"
    volumes:
      - ./config.yaml:/app/etc/config.yaml:ro
      - app-data:/app/data
    depends_on:
      - db
    restart: unless-stopped

  db:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: birthday
      POSTGRES_PASSWORD: birthday
      POSTGRES_DB: birthday_reminders
    volumes:
      - pg-data:/var/lib/postgresql/data
    restart: unless-stopped

volumes:
  app-data:
  pg-data:
```

### Using Make

```bash
# Build and install to /opt/birthday-reminders (default)
make install

# Or install to a custom prefix
make install PREFIX=/usr/local
```

This installs:
- Binary → `<prefix>/bin/birthday-reminders`
- Config → `<prefix>/etc/config.yaml`
- Migrations → `<prefix>/share/migrations/`
- Static files → `<prefix>/share/static/`

### Package & Deploy

```bash
# Create a distributable tar.gz
make package

# Copy to target server, extract, and install
scp target/package/birthday-reminders-*.tar.gz user@server:/tmp/
ssh user@server 'cd /tmp && tar xzf birthday-reminders-*.tar.gz && cd birthday-reminders-* && sudo ./install.sh'
```

The `install.sh` script automatically detects systemd or OpenRC, creates a service user, and installs the appropriate service file.

### systemd (manual)

```bash
# Create service user
useradd -r -s /usr/sbin/nologin -d /opt/birthday-reminders birthday-reminders

# Install and enable the service
cp package/systemd/birthday-reminders.service /etc/systemd/system/
systemctl daemon-reload
systemctl enable --now birthday-reminders
```

### Alpine Linux / OpenRC (manual)

```bash
# Create service user
adduser -S -D -H -h /opt/birthday-reminders -s /sbin/nologin birthday-reminders
addgroup -S birthday-reminders

# Install and enable the service
cp package/openrc/birthday-reminders.openrc /etc/init.d/birthday-reminders
chmod +x /etc/init.d/birthday-reminders
rc-update add birthday-reminders default
rc-service birthday-reminders start
```

When installed to `/opt/birthday-reminders`, the default config uses absolute paths:

```yaml
database:
  url: "sqlite:///opt/birthday-reminders/data/birthday_reminders.db"

server:
  static_dir: "/opt/birthday-reminders/static"
```

### Directory Layout

```
/opt/birthday-reminders/
├── bin/            # Binary
├── etc/            # Configuration (config.yaml)
├── data/           # Runtime data (SQLite database)
├── migrations/     # SQL migration files
└── static/         # Static assets (CSS, JS, manifest)
```

## Backup & Restore

### SQLite

```bash
# Backup (while the app is running — SQLite WAL mode allows safe reads)
cp /opt/birthday-reminders/data/birthday_reminders.db /backups/birthday_reminders_$(date +%F).db

# Or use the SQLite backup command for a consistent snapshot
sqlite3 /opt/birthday-reminders/data/birthday_reminders.db ".backup /backups/birthday_reminders_$(date +%F).db"

# Restore
systemctl stop birthday-reminders
cp /backups/birthday_reminders_2026-04-29.db /opt/birthday-reminders/data/birthday_reminders.db
systemctl start birthday-reminders
```

### PostgreSQL

```bash
# Backup
pg_dump birthday_reminders > /backups/birthday_reminders_$(date +%F).sql

# Restore
systemctl stop birthday-reminders
dropdb birthday_reminders
createdb birthday_reminders
psql birthday_reminders < /backups/birthday_reminders_2026-04-29.sql
systemctl start birthday-reminders
```

### What to back up

- **Database** — contains all user data, birthdays, and notification configs
- **`config.yaml`** — contains your secrets (session_secret, encryption_key, OIDC credentials)

The binary, static files, and migrations can be rebuilt from source.

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.
