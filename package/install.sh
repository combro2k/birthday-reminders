#!/bin/sh
set -e

PREFIX="${PREFIX:-/opt/birthday-reminders}"
BINDIR="$PREFIX/bin"
CONFDIR="$PREFIX/etc"
DATADIR="$PREFIX/data"
MIGRATIONSDIR="$PREFIX/migrations"
STATICDIR="$PREFIX/static"

echo "Installing birthday-reminders to $PREFIX ..."

# Create directories
install -d "$BINDIR"
install -d "$CONFDIR"
install -d "$DATADIR"
install -d "$MIGRATIONSDIR"
install -d "$STATICDIR"

# Install binary
install -m 755 birthday-reminders "$BINDIR/birthday-reminders"

# Install config (don't overwrite existing)
if [ ! -f "$CONFDIR/config.yaml" ]; then
    install -m 640 config.yaml.example "$CONFDIR/config.yaml"
    echo "  Installed default config to $CONFDIR/config.yaml"
    echo "  >>> Edit this file before starting the service! <<<"
else
    install -m 640 config.yaml.example "$CONFDIR/config.yaml.new"
    echo "  Config exists, new version saved as $CONFDIR/config.yaml.new"
fi

# Install data files
cp -r migrations/* "$MIGRATIONSDIR/"
cp -r static/* "$STATICDIR/"

# Create service user if it doesn't exist
if ! id birthday-reminders >/dev/null 2>&1; then
    if command -v useradd >/dev/null 2>&1; then
        useradd -r -s /usr/sbin/nologin -d "$PREFIX" birthday-reminders
    elif command -v adduser >/dev/null 2>&1; then
        addgroup -S birthday-reminders 2>/dev/null || true
        adduser -S -D -H -h "$PREFIX" -s /sbin/nologin -G birthday-reminders birthday-reminders
    fi
    echo "  Created service user: birthday-reminders"
fi

# Set ownership
chown -R birthday-reminders: "$PREFIX" 2>/dev/null || true

# Install service file
if [ -d /run/systemd/system ]; then
    cp package/systemd/birthday-reminders.service /etc/systemd/system/
    systemctl daemon-reload
    echo "  Installed systemd service (enable with: systemctl enable --now birthday-reminders)"
elif [ -d /etc/init.d ]; then
    cp package/openrc/birthday-reminders.openrc /etc/init.d/birthday-reminders
    chmod +x /etc/init.d/birthday-reminders
    echo "  Installed OpenRC service (enable with: rc-update add birthday-reminders default)"
fi

echo ""
echo "Installation complete!"
echo "  Binary:     $BINDIR/birthday-reminders"
echo "  Config:     $CONFDIR/config.yaml"
echo "  Data:       $DATADIR/"
echo "  Migrations: $MIGRATIONSDIR/"
echo "  Static:     $STATICDIR/"
echo ""
echo "Next steps:"
echo "  1. Edit $CONFDIR/config.yaml"
echo "  2. Start the service or run: $BINDIR/birthday-reminders -c $CONFDIR/config.yaml serve"
