#!/usr/bin/env bash
set -euo pipefail

export PORT="${PORT:-4444}"
export HOST="${HOST:-localhost}"
export CALLBACK_URL="${CALLBACK_URL:-http://localhost:3001/on-doc-update}"
export CALLBACK_DEBOUNCE_WAIT="${CALLBACK_DEBOUNCE_WAIT:-500}"
export CALLBACK_DEBOUNCE_MAXWAIT="${CALLBACK_DEBOUNCE_MAXWAIT:-2000}"

echo "Starting y-websocket on :${PORT} (callback → $CALLBACK_URL)"
npx y-websocket
