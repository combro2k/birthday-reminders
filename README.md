# Birthday Reminders

A self-hosted birthday reminder application with a web UI, CLI, and flexible notification channels. Built with Rust, Axum, and SQLx.

## Features

- **Web UI** — manage birthdays, contact details, notification channels, and user settings
- **CLI** — add/list/remove birthdays with contact details and manage users from the terminal
- **Notifications** — Email, Gotify, Telegram, Signal, and WhatsApp
- **Scheduled reminders** — configurable cron schedule with customizable lead days
- **Authentication** — local accounts with Argon2 password hashing, or OIDC (Keycloak, Authentik, Zitadel, and others)
- **Encryption** — notification channel secrets encrypted at rest using XChaCha20-Poly1305
- **API tokens** — programmatic access for automation
- **PWA support** — installable as a progressive web app

## Prerequisites

### Runtime Requirements
- **Rust** 1.85+ (edition 2024)
- **Node.js** 20+ and **npm** (for Tailwind CSS build)
- **PostgreSQL** 13+, **MySQL** 8.0+, or **SQLite** 3.35+
- A running database instance (for PostgreSQL or MySQL)

### Build & Development Tools
- **cargo** (Rust package manager, included with Rust)
- **git** (version control)
- **make** (optional, simplifies build commands)
- **npm** (included with Node.js)
- **gitleaks** (for secret scanning; install with `cargo install gitleaks` or use your package manager)
- **openssl** (for generating encryption keys)
- **python3** (for PWA asset generation script)
- **mold** (optional, fast linker for faster builds)
- **sccache** (optional, shared compilation cache to speed up rebuilds)
- **clang** (C compiler, required for linking)

### Optional
- **Docker** and **docker-compose** (for containerized deployments)

## Quick Start

### 1. Build

```bash
npm install
npm run build:css
cargo build --release
```

You can also run `make build`, which now builds Tailwind CSS before compiling Rust.

### 2. Configure

```bash
sudo install -d -m 755 /etc/birthday-reminders
sudo install -m 640 config.yaml.example /etc/birthday-reminders/config.yaml
# Edit /etc/birthday-reminders/config.yaml with your settings
```

### 3. Create a user

```bash
./target/release/birthday-reminders create-user \
  --username admin \
  --email admin@example.com \
  --password changeme \
  --admin
```

### 4. Start the server

```bash
./target/release/birthday-reminders serve
```

The app will be available at `http://localhost:3000` by default.

### PWA assets

Android installability assets (icons and screenshots) are generated with:

```bash
python3 scripts/generate_pwa_assets.py
```

The generated files are written to `static/` and referenced by `static/manifest.json`.

## Configuration

Configuration is stored in a YAML file (default: `/etc/birthday-reminders/config.yaml`). See [`config.yaml.example`](config.yaml.example) for a fully commented example.

### Database

Supports SQLite, MySQL, and PostgreSQL:

```yaml
database:
  # SQLite (default, simplest for single-user/small deployments)
  url: "sqlite:///var/lib/birthday-reminders/birthday_reminders.db?mode=rwc"
  max_connections: 10
```

```yaml
database:
  # MySQL
  url: "mysql://birthday:birthday@localhost:3306/birthday_reminders"
  max_connections: 10
```

```yaml
database:
  # PostgreSQL (recommended for multi-user or production deployments)
  url: "postgres://birthday:birthday@localhost:5432/birthday_reminders"
  max_connections: 10
```

Database migrations run automatically on startup.
Migrations are split per backend under `migrations/sqlite`, `migrations/mysql`, and `migrations/postgres`.
Each backend track has:
- an `init` migration for clean installs,
- incremental migrations for each schema change.

- For SQLite, use a URL like:

  ```yaml
  database:
    url: "sqlite:///var/lib/birthday-reminders/birthday_reminders.db?mode=rwc"
  ```
  The `?mode=rwc` ensures the database file is created if it does not exist.

### Migrating from SQLite to PostgreSQL

If you started with SQLite and want to move to PostgreSQL:

1. **Export data from SQLite:**

   ```bash
  sqlite3 /var/lib/birthday-reminders/birthday_reminders.db .dump > backup.sql
   ```

2. **Create the PostgreSQL database:**

   ```bash
   createdb birthday_reminders
   ```

3. **Start the app once with the new PostgreSQL URL** to run migrations:

   ```bash
  birthday-reminders serve
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

5. **Update `/etc/birthday-reminders/config.yaml`** to use the PostgreSQL URL and restart.

### Server

```yaml
server:
  listen: "0.0.0.0:3000"
  server_name: "birthdays.example.com"
  scheme: "https"
  # Optional override when the externally visible URL differs from scheme + server_name
  # base_url: "https://birthdays.example.com/app"
  session_secret: "generate-a-random-string-at-least-32-chars"
  encryption_key: "generate-a-separate-key-for-encryption"  # required, generate with: openssl rand -base64 32
  # Optional: trust forwarded headers from these proxy IPs or CIDRs only
  # trusted_proxies: ["127.0.0.1", "10.0.0.0/8"]
```

> **Important:** Generate a strong random `session_secret` for production. It must be at least 32 characters.
>
> The `encryption_key` is used to encrypt notification channel secrets at rest (XChaCha20-Poly1305). Generate a dedicated key with: `openssl rand -base64 32`

`scheme` + `server_name` define the public URL used for OIDC callbacks, generated links, and secure-cookie detection. Set `base_url` only when the public URL cannot be expressed as `scheme://server_name`, such as a reverse proxy that serves the app from a sub-path.

When the app is behind a reverse proxy, add the proxy IPs or CIDRs to `trusted_proxies`. Forwarded headers from any other peer are ignored, so clients cannot spoof their rate-limit identity with `X-Forwarded-For`.

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
| `trusted_audiences` | No | `[]` | Additional trusted `aud` values when ID tokens include multiple audiences |
| `allow_dynamic_additional_audiences` | No | `false` | Trust all additional `aud` values; keep `client_id`/`azp` checks enforced |
| `auto_provision` | No | `true` | Create local accounts on first OIDC login |
| `default_role` | No | `"user"` | Role assigned to auto-provisioned users |

The callback URL to configure in your provider is:

```
{resolved_public_url}/auth/oidc/callback
```

If `base_url` is set, it is used directly. Otherwise the app derives the callback URL from `scheme://server_name`.

For example: `https://birthdays.example.com/auth/oidc/callback`

If your provider returns ID tokens with more than one audience, keep `client_id` set to your app's client ID and add the other allowed audience values to `trusted_audiences`.

If additional audiences are provider-managed and change over time (common with some self-hosted Zitadel setups), set `allow_dynamic_additional_audiences: true`.

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
    allow_dynamic_additional_audiences: true
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
  -c, --config <PATH>  Path to config file [default: /etc/birthday-reminders/config.yaml]
```

Commands that require `--token` also accept the `BIRTHDAY_API_TOKEN` environment variable:

```bash
export BIRTHDAY_API_TOKEN=your-api-token
birthday-reminders list
```

The `serve` command accepts a `--port` (`-p`) flag to override the listen port from the config:

```bash
# Start on a custom port
birthday-reminders serve --port 8080
```

### Examples

```bash
# Add a birthday
birthday-reminders add "Jane Doe" 1990-05-15 --token your-api-token

# Add a birthday with contact details
birthday-reminders add "Jane Doe" 1990-05-15 \
  --phone-number "+31 6 12345678" \
  --address "Keizersgracht 1" \
  --postal-code "1015 CC" \
  --city "Amsterdam" \
  --country "Netherlands" \
  --token your-api-token

# Add a birthday with notes
birthday-reminders add "John Smith" 1985-12-25 --notes "Likes chocolate" --token your-api-token

# Add a birthday with both notes and address data
birthday-reminders add "John Smith" 1985-12-25 \
  --phone-number "+1 555 123 4567" \
  --address "123 Main St" \
  --postal-code "90210" \
  --city "Beverly Hills" \
  --country "USA" \
  --notes "Likes chocolate" \
  --token your-api-token

# List upcoming birthdays in the next 14 days (default: 30)
birthday-reminders upcoming --days 14 --token your-api-token

# Manually trigger reminders (useful for testing)
birthday-reminders check-reminders
```

The `add` command accepts these optional contact fields:

- `--phone-number`
- `--address`
- `--postal-code`
- `--city`
- `--country`

These same fields are also available in the web form when creating or editing a birthday.

## Notification Channels

Users can configure one or more notification channels in the web UI under **Settings → Notification Channels**:

| Channel | Description |
|---------|-------------|
| **Email** | SMTP-based email notifications |
| **Gotify** | Push notifications via a Gotify server |
| **Telegram** | Messages via Telegram Bot API |
| **Signal** | Messages via Signal messenger |
| **WhatsApp** | Messages via WhatsApp Business API |

### Proton Mail Email Setup

Email channels support two Proton modes:

1. Proton SMTP Submission (recommended)
2. Proton Bridge (local bridge)

#### Proton SMTP Submission (recommended)

- Provider: `proton_smtp`
- Host: `smtp.protonmail.ch`
- Port: `587`
- Security: `STARTTLS`
- Username: your Proton email address
- Password: your generated SMTP token

Example:

```json
{
  "provider": "proton_smtp",
  "username": "you@proton.me",
  "password": "your-smtp-token",
  "to": "you@proton.me"
}
```

#### Proton Bridge (local)

- Provider: `proton`
- Host: `127.0.0.1`
- Port: `1025`
- Security: `STARTTLS`

Example:

```json
{
  "provider": "proton",
  "username": "you@proton.me",
  "password": "bridge-password",
  "to": "you@proton.me"
}
```

Proton SMTP uses SMTP token credentials. Do not use your Proton account login password in third-party SMTP clients.

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

After starting the containers, create your first admin user:

```bash
docker compose exec app /app/bin/birthday-reminders \
  -c /app/etc/config.yaml create-user \
  --username admin \
  --email admin@example.com \
  --password changeme \
  --admin
```

### Using Make

```bash
# Build and install to system paths (default)
make install

# Or stage files for packaging with DESTDIR
make install DESTDIR=/tmp/birthday-reminders-stage
```

This installs:
- Binary → `/usr/bin/birthday-reminders`
- Config → `/etc/birthday-reminders/config.yaml`
- Data directory → `/var/lib/birthday-reminders/`
- Migrations → (Embedded in binary)
- Static files → (Embedded in binary)

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
useradd -r -s /usr/sbin/nologin -d /var/lib/birthday-reminders birthday-reminders

# Install and enable the service
cp package/systemd/birthday-reminders.service /etc/systemd/system/
systemctl daemon-reload
systemctl enable --now birthday-reminders
```

### Alpine Linux / OpenRC (manual)

```bash
# Create service user
adduser -S -D -H -h /var/lib/birthday-reminders -s /sbin/nologin birthday-reminders
addgroup -S birthday-reminders

# Install and enable the service
cp package/openrc/birthday-reminders.openrc /etc/init.d/birthday-reminders
chmod +x /etc/init.d/birthday-reminders
rc-update add birthday-reminders default
rc-service birthday-reminders start
```

For service-style Linux installs, the default config uses absolute paths:

```yaml
database:
  url: "sqlite:///var/lib/birthday-reminders/birthday_reminders.db?mode=rwc"

server:
  static_dir: "/var/lib/birthday-reminders/static"
```

### Directory Layout

```
/usr/bin/
└── birthday-reminders

/etc/birthday-reminders/
└── config.yaml

/var/lib/birthday-reminders/
└── birthday_reminders.db
```

## Backup & Restore

### SQLite

```bash
# Backup (while the app is running — SQLite WAL mode allows safe reads)
cp /var/lib/birthday-reminders/birthday_reminders.db /backups/birthday_reminders_$(date +%F).db

# Or use the SQLite backup command for a consistent snapshot
sqlite3 /var/lib/birthday-reminders/birthday_reminders.db ".backup /backups/birthday_reminders_$(date +%F).db"

# Restore
systemctl stop birthday-reminders
cp /backups/birthday_reminders_2026-04-29.db /var/lib/birthday-reminders/birthday_reminders.db
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

## Release Checklist

Before publishing a new version, run the release check script:

```bash
bash scripts/release-check.sh
```

This script performs the following checks in order:
1. **Version consistency** — verifies `Cargo.toml` and `package.json` have matching versions
2. **Conditional clean** — runs `cargo clean` only when Git changes are present in `src/`, `static/`, `templates/`, `tests/`, or `migrations/`
3. **Secret scanning** — runs `gitleaks detect` to ensure no credentials, tokens, or API keys are present
4. **CSS build** — rebuilds Tailwind CSS
5. **Code formatting** — runs `cargo fmt --all`
6. **Tests** — runs `cargo test --all-targets --all-features`
7. **Linting** — runs `cargo clippy` with strict warnings-as-errors mode

All checks must pass without errors before release.

Also ensure:
- Changes are documented in [CHANGELOG.md](CHANGELOG.md).
- The codebase does not contain personal/private information or secrets (tokens, passwords, API keys, credentials, etc.).

Full release workflow requirements are enforced in [AGENTS.md](AGENTS.md).

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.

## Configuration Defaults

The following configuration fields have defaults and are optional unless marked as required:

| Field | Default | Description |
|-------|---------|-------------|
| `database.max_connections` | `10` | Max DB connections |
| `server.scheme` | `"http"` | Public URL scheme used with `server_name` |
| `server.static_dir` | `/var/lib/birthday-reminders/static` | Path to static files |
| `server.trusted_proxies` | `[]` | Proxy IPs/CIDRs allowed to supply forwarded client IP headers |
| `auth.allow_registration` | `false` | Allow user self-registration |
| `auth.oidc.enabled` | `false` | Enable OIDC authentication |
| `auth.oidc.scopes` | `["openid", "profile", "email"]` | OIDC scopes |
| `auth.oidc.auto_provision` | `true` | Auto-provision OIDC users |
| `auth.oidc.default_role` | `"user"` | Default OIDC user role |
| `reminders.schedule` | `"0 0 8 * * *"` | Reminder cron schedule |
| `reminders.default_days_before` | `[7, 3, 1, 0]` | Days before birthday to remind |
| `logging.output` | `"stdout"` | Log output target |
| `logging.level` | `"info"` | Log level filter |

All path defaults (such as `static_dir`) are now absolute by default, e.g. `/var/lib/birthday-reminders/static`.

`server.base_url` is optional. When omitted, the app derives the public URL from `server.scheme://server.server_name`. `server.server_name` itself has no default and must be provided whenever `server.base_url` is not set.

See [`config.yaml.example`](config.yaml.example) for a fully commented example with all defaults and required fields clearly marked.
