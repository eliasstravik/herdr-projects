#!/bin/sh
# Open or focus the single Mission Control workspace, with project list, task input, and visible CLI panes.
set -eu
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec sh "$SCRIPT_DIR/../../bin/run.sh" "${1:-control-open}"
