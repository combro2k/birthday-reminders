#!/bin/sh
set -e

if command -v rc-service >/dev/null 2>&1; then
    rc-service birthday-reminders stop 2>/dev/null || true
fi

if command -v rc-update >/dev/null 2>&1; then
    rc-update del birthday-reminders 2>/dev/null || true
fi
