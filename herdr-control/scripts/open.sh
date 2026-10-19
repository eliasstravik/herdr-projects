#!/bin/sh
# Open Herdr Control's declared split-pane entrypoint.
set -eu
HERDR_BIN=${HERDR_BIN_PATH:-herdr}
exec "$HERDR_BIN" plugin pane open --plugin herdr-control --entrypoint control --placement split --direction right
