#!/bin/sh
# The workspace setup (.herdr-projects.json): run in every fresh worktree before its agent starts. It installs the
# build cache all worktrees share (scripts/build-cache.sh) and mold, the quicker linker (scripts/linker.sh), starts
# from the warm build of the main line (scripts/warm-start.sh), builds the program through the cache with the `quick`
# profile on its share of the cores, counted among the checks and setups running now (scripts/share-cores.sh), keeps
# that build warm for the next worktree, and writes the made pages, the map among them.
set -e
cd "$(dirname -- "$0")/.."
sh scripts/build-cache.sh
sh scripts/linker.sh --install
. scripts/build-cache.sh
. scripts/share-cores.sh
sc_join
trap 'sc_leave' EXIT
. scripts/warm-start.sh
ws_seed
sc_cargo build --profile quick
ws_keep
./target/quick/herdr-projects docs
