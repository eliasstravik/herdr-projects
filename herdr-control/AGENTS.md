# Herdr Control — Entwicklerhinweise

- Plugin-Manifest: `herdr-plugin.toml`; jeder direkte Ordner unter `~/Developer` erhält einen eigenen Herdr-Space mit fest gebundenem Taskfeld. Space-Wechsel ersetzt keine Sitzung.
- Der erste Task startet den sichtbaren interaktiven Orchestrator im selben Space; Worker folgen als echte Split-Panes. Die Task-Eingabe bleibt bestehen.
- `scripts/control.sh` speichert die globale Reihenfolge unter `HERDR_PLUGIN_CONFIG_DIR` und schreibt sie über `herdr-projects set`.
- Agentenliste muss mit den unterstützten Einträgen in Herdr Projects synchron bleiben: Freebuff, CodeBuddy, Claude Code, Codex, Gemini CLI, Cursor, OpenCode, Kiro, Cline, Qoder CLI (`qodercli`) und Grok.
- Keine Ordnerauswahl-Liste in Mission Control hinzufügen; Herdr-Spaces sind die Projektauswahl. Keine ACP- oder Headless-Steuerung hinzufügen. Keine Agenten in zusätzlichen Tabs starten.
- Nach Funktionsänderungen README und diese Datei aktuell halten.
