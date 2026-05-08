#!/bin/sh
set -e

# Create service user if absent
if ! id birthday-reminders >/dev/null 2>&1; then
    useradd --system --no-create-home --shell /usr/sbin/nologin birthday-reminders
fi

# Create required directories
install -d -m 755 /etc/birthday-reminders
install -d -m 750 -o birthday-reminders -g birthday-reminders /var/lib/birthday-reminders

# Seed config from example if no config exists yet
if [ ! -f /etc/birthday-reminders/config.yaml ]; then
    install -m 640 -o root -g birthday-reminders \
        /etc/birthday-reminders/config.yaml.example \
        /etc/birthday-reminders/config.yaml
    echo "  Config installed to /etc/birthday-reminders/config.yaml"
    echo "  >>> Edit this file before starting the service! <<<"
fi

# Reload systemd so the new unit is picked up
if command -v systemctl >/dev/null 2>&1 && systemctl is-system-running --quiet 2>/dev/null; then
    systemctl daemon-reload
fi
