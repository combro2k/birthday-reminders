# Birthday Reminders

A self-hosted birthday reminder application with a web UI, CLI, and flexible notification channels. Built with Rust, Axum, and SQLx.

## Features

- **Web UI** — manage birthdays, notification channels, and user settings
- **CLI** — add/list/remove birthdays and manage users from the terminal
- **Notifications** — Email, Gotify, Telegram, Signal, and WhatsApp
- **Scheduled reminders** — configurable cron schedule with customizable lead days
- **Authentication** — local accounts with Argon2 password hashing, or OIDC (Keycloak, Authentik, Zitadel, and others)
- **API tokens** — programmatic access for automation
- **PWA support** — installable as a progressive web app

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

### Server

```yaml
server:
  listen: "0.0.0.0:3000"
  base_url: "http://localhost:3000"
  session_secret: "generate-a-random-64-char-string-here"
  static_dir: "static"  # path to static assets (CSS, JS, manifest)
```

> **Important:** Generate a strong random `session_secret` for production. It must be at least 32 characters.

### Reminders

```yaml
reminders:
  # Cron expression: sec min hour day month weekday
  schedule: "0 0 8 * * *"       # Every day at 08:00
  default_days_before: [7, 3, 1, 0]  # Remind 7, 3, and 1 day(s) before + on the day
```

The `default_days_before` setting is the global default. Each user can override this from **Settings → Profile → Reminder Preferences** in the web UI, choosing from 14, 7, 3, or 1 day(s) before and/or on the day itself.

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

### systemd

```bash
# Create service user
useradd -r -s /usr/sbin/nologin -d /opt/birthday-reminders birthday-reminders

# Install and enable the service
cp dist/birthday-reminders.service /etc/systemd/system/
systemctl daemon-reload
systemctl enable --now birthday-reminders
```

### Alpine Linux (OpenRC)

```bash
# Create service user
adduser -S -D -H -h /opt/birthday-reminders -s /sbin/nologin birthday-reminders
addgroup -S birthday-reminders

# Install and enable the service
cp dist/birthday-reminders.openrc /etc/init.d/birthday-reminders
chmod +x /etc/init.d/birthday-reminders
rc-update add birthday-reminders default
rc-service birthday-reminders start
```

When installed to `/opt/birthday-reminders`, set `static_dir` in your config:

```yaml
server:
  static_dir: "/opt/birthday-reminders/share/static"
```

## License

See [LICENSE](LICENSE) for details.
