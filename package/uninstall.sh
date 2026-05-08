#!/bin/sh
set -e

BINDIR="${BINDIR:-/usr/bin}"
CONFDIR="${CONFDIR:-/etc/birthday-reminders}"
DATADIR="${DATADIR:-/var/lib/birthday-reminders}"
STATICDIR="${STATICDIR:-$DATADIR/static}"

# Remove installed files and directories
rm -f "$BINDIR/birthday-reminders"
rm -rf "$STATICDIR"
rm -rf "$DATADIR"

# Optionally remove config (uncomment to enable)
# rm -rf "$CONFDIR"

echo "Uninstall complete. Config at $CONFDIR was preserved. Remove manually if desired."
