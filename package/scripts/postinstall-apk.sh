#!/bin/sh
set -e

# Create service user and group if absent
if ! id birthday-reminders >/dev/null 2>&1; then
    addgroup -S birthday-reminders 2>/dev/null || true
    adduser -S -D -H -s /sbin/nologin -G birthday-reminders birthday-reminders
fi

# Create required directories
install -d -m 755 /etc/birthday-reminders
install -d -m 750 /var/lib/birthday-reminders
chown birthday-reminders:birthday-reminders /var/lib/birthday-reminders

# Seed config from example if no config exists yet
if [ ! -f /etc/birthday-reminders/config.yaml ]; then
    install -m 640 /etc/birthday-reminders/config.yaml.example \
        /etc/birthday-reminders/config.yaml
    chown root:birthday-reminders /etc/birthday-reminders/config.yaml
    echo "  Config installed to /etc/birthday-reminders/config.yaml"
    echo "  >>> Edit this file before starting the service! <<<"
fi
