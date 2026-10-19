#!/bin/sh
# Edit and apply the global CLI priority list for Herdr Projects.
set -eu

CONFIG_DIR=${HERDR_PLUGIN_CONFIG_DIR:-${XDG_CONFIG_HOME:-$HOME/.config}/herdr/plugins/herdr-control}
PRIORITY_FILE="$CONFIG_DIR/priority.txt"
PROJECTS_BIN=${HERDR_PROJECTS_BIN:-herdr-projects}
mkdir -p "$CONFIG_DIR"

show_order() {
  if [ -s "$PRIORITY_FILE" ]; then
    cat "$PRIORITY_FILE"
  else
    printf '%s\n' 'Noch keine Reihenfolge gespeichert.'
  fi
}

validate_order() {
  python3 - "$1" <<'PY'
import sys
allowed = {"freebuff", "codebuddy", "claude", "codex", "gemini", "cursor", "opencode", "kiro", "cline", "qodercli", "grok"}
items = [part.strip() for part in sys.argv[1].split(",") if part.strip()]
if not items:
    raise SystemExit("Bitte mindestens eine CLI angeben.")
seen = set()
for item in items:
    name = item.split("/", 1)[0]
    if name not in allowed:
        raise SystemExit(f"Unbekannte CLI '{name}'. Erlaubt: {', '.join(sorted(allowed))}")
    if item in seen:
        raise SystemExit(f"Doppelter Eintrag: {item}")
    if any(ch.isspace() for ch in item):
        raise SystemExit("Einträge dürfen keine Leerzeichen enthalten; Modelle als cli/modell schreiben.")
    seen.add(item)
print(",".join(items))
PY
}

apply_order() {
  order=$(cat "$PRIORITY_FILE")
  primary=${order%%,*}
  fallback=${order#*,}
  if [ "$fallback" = "$order" ]; then fallback=none; fi
  "$PROJECTS_BIN" set worker.fallback "$fallback"
  "$PROJECTS_BIN" set worker.agent "$primary"
  printf '\nAktiviert: %s → Fallback: %s\n' "$primary" "$fallback"
}

while :; do
  printf '\nHerdr Control — globale Agenten-Priorität\n\n'
  printf 'Aktuelle Reihenfolge: '
  show_order
  printf '\n1) Reihenfolge bearbeiten\n2) Auf Herdr Projects anwenden\n3) Beenden\nAuswahl: '
  IFS= read -r choice || exit 0
  case "$choice" in
    1)
      printf 'CLIs/Modelle in Reihenfolge, kommasepariert (z. B. claude/opus,codex,cline,qodercli,grok):\n> '
      IFS= read -r value || exit 0
      normalized=$(validate_order "$value") || { printf '\nEingabe nicht übernommen.\n'; continue; }
      tmp="$PRIORITY_FILE.tmp.$$"
      printf '%s\n' "$normalized" > "$tmp"
      mv "$tmp" "$PRIORITY_FILE"
      printf '\nGespeichert. Noch nicht auf Herdr Projects angewendet.\n'
      ;;
    2)
      if [ ! -s "$PRIORITY_FILE" ]; then
        printf '\nZuerst eine Reihenfolge speichern.\n'
      else
        apply_order
      fi
      ;;
    3) exit 0 ;;
    *) printf '\nBitte 1, 2 oder 3 wählen.\n' ;;
  esac
done
