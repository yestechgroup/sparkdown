#!/usr/bin/env bash
set -euo pipefail

export CALLBACK_URL="${CALLBACK_URL:-http://localhost:3001/on-doc-update}"
export CALLBACK_DEBOUNCE_WAIT="${CALLBACK_DEBOUNCE_WAIT:-500}"
export CALLBACK_DEBOUNCE_MAXWAIT="${CALLBACK_DEBOUNCE_MAXWAIT:-2000}"

echo "Starting y-websocket on :4444 (callback → $CALLBACK_URL)"
npx y-websocket --port 4444
