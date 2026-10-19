# Herdr Control — Entwicklerhinweise

- Plugin-Manifest: `herdr-plugin.toml`; die einzige Agenten-UI ist ein Split-Pane.
- `scripts/control.sh` speichert die globale Reihenfolge unter `HERDR_PLUGIN_CONFIG_DIR` und schreibt sie über `herdr-projects set`.
- Agentenliste muss mit den unterstützten Einträgen in Herdr Projects synchron bleiben: Freebuff, CodeBuddy, Claude Code, Codex, Gemini CLI, Cursor, OpenCode, Kiro, Cline, Qoder CLI (`qodercli`) und Grok.
- Keine ACP- oder Headless-Steuerung hinzufügen. Keine Agenten in Tabs starten.
- Nach Funktionsänderungen README und diese Datei aktuell halten.
