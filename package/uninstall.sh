#!/bin/sh
set -e

PREFIX="${PREFIX:-/opt/birthday-reminders}"
BINDIR="$PREFIX/bin"
CONFDIR="$PREFIX/etc"
DATADIR="$PREFIX/data"
STATICDIR="$PREFIX/static"

# Remove installed files and directories
rm -f "$BINDIR/birthday-reminders"
rm -rf "$STATICDIR"
rm -rf "$DATADIR"

# Optionally remove config (uncomment to enable)
# rm -rf "$CONFDIR"

echo "Uninstall complete. Config at $CONFDIR was preserved. Remove manually if desired."
