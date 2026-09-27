#!/bin/sh
# Puts `herdr-projects` on PATH during `herdr plugin install`.
#
# Herdr runs the build in <plugins>/.tmp-install-*/checkout and, once it
# passes, moves the checkout to <plugins>/github/<id>-<first 12 hex digits of
# sha256(id)>. This links $XDG_BIN_HOME/herdr-projects (else
# ~/.local/bin/herdr-projects) to the binary's final place there, so the
# command works as soon as the install finishes. Anywhere else it does
# nothing: the binary itself relinks at every plugin start and on
# `doctor --fix`. It never replaces a file, or a link that points outside
# Herdr's plugin folder and still works.
#
#   sh scripts/link-command.sh [checkout]   (default: the current folder)
set -u

say() { printf 'herdr-projects install: %s\n' "$*" >&2; }

checkout=${1:-$PWD}
case "$checkout" in
  */plugins/.tmp-install-*/checkout) ;;
  *) exit 0 ;;
esac
plugins=${checkout%/.tmp-install-*/checkout}
hash=$(printf %s herdr-projects | { sha256sum 2>/dev/null || shasum -a 256; } | cut -c 1-12)
[ ${#hash} -eq 12 ] || exit 0
target="$plugins/github/herdr-projects-$hash/target/release/herdr-projects"
bin=${XDG_BIN_HOME:-$HOME/.local/bin}
link="$bin/herdr-projects"

if [ -L "$link" ]; then
  current=$(readlink "$link")
  case "$current" in
    "$plugins"/*) ;;
    *) if [ -e "$link" ]; then
         say "left $link alone: it links $current"
         exit 0
       fi ;;
  esac
elif [ -e "$link" ]; then
  say "left $link alone: it is not a link"
  exit 0
fi
if ! mkdir -p "$bin" || ! ln -sfn "$target" "$link"; then
  say "could not link $link"
  exit 0
fi
case ":${PATH:-}:" in
  *":$bin:"*) say "linked $link" ;;
  *) say "linked $link, but $bin is not on your PATH: add it in your shell profile" ;;
esac
