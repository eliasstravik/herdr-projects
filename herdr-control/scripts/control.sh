#!/bin/sh
# Run the persistent native Mission Control UI from this feature's own Projects build.
set -eu
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec sh "$SCRIPT_DIR/../../bin/run.sh" control
