# Herdr Control

Ein kleines Herdr-Plugin zur Pflege einer globalen Reihenfolge von Coding-CLIs und Modellen. Die erste Position wird `worker.agent`; alle folgenden Positionen werden `worker.fallback` in Herdr Projects. Herdr Projects übernimmt Task-Verteilung, Worktrees, Agentenstatus und den Fallback bei erkannten Provider-/Kontingentfehlern.

## Unterstützte CLIs

`freebuff`, `codebuddy`, `claude`, `codex`, `gemini`, `cursor`, `opencode`, `kiro`, `cline`, `qodercli` (Qoder CLI) und `grok`. Ein Modell kann als `cli/modellname` angegeben werden. Modellnamen müssen vom jeweiligen CLI akzeptiert werden.

## Installation

Direkt aus dem Fork installieren:

```sh
herdr plugin install ysamjo/herdr-projects/herdr-control --ref feature/interactive-cli-pool
```

Nach dem Merge in den Standardbranch entfällt `--ref feature/interactive-cli-pool`. Für lokale Entwicklung kann das Verzeichnis verlinkt werden:

```sh
herdr plugin link /absoluter/pfad/zu/herdr-control
```

Danach in Herdr die Aktion **Herdr Control öffnen** ausführen. Das Control-Fenster öffnet sich als Split-Pane. Für die Projects-Integration muss `herdr-projects` über die Shell-PATH erreichbar sein; alternativ kann `HERDR_PROJECTS_BIN` auf das Programm zeigen.

## Herdr-Tastenkürzel

Die globale Herdr-Konfiguration verwendet den Prefix `§`, Kurzbefehle zum Wechseln zwischen Tabs, Workspaces und Panes sowie einen Lazygit-Popup im fokussierten Pane. Neue Agenten werden davon unabhängig als Split-Panes im bestehenden Tab gestartet.

```toml
[keys]
prefix = "§"
new_tab = ["prefix+c", "cmd+t"]
close_tab = ["prefix+shift+x", "cmd+w"]
goto = ["prefix+g", "cmd+k"]
previous_tab = ["prefix+p", "ctrl+left"]
next_tab = ["prefix+n", "ctrl+right"]
previous_workspace = "ctrl+up"
next_workspace = "ctrl+down"
cycle_pane_next = ["prefix+tab", "alt+up"]
cycle_pane_previous = ["prefix+shift+tab", "alt+down"]

[[keys.command]]
key = ["cmd+shift+g", "prefix+d"]
type = "popup"
command = 'cd "${HERDR_ACTIVE_PANE_CWD:-.}" && exec lazygit'
description = "Git status"
width = "90%"
height = "85%"
```

Diese Einträge gehören in `~/.config/herdr/config.toml`; bestehende `[keys]`-Einträge dort zusammenführen, nicht doppelt anlegen.

## Bedienung

1. Reihenfolge bearbeiten, z. B. `claude/opus,codex,gemini,cline,qodercli,grok`.
2. „Auf Herdr Projects anwenden“ wählen.
3. Herdr Projects setzt den ersten Eintrag global als primären Worker und die restlichen als geordnete Fallbacks.

Die Reihenfolge wird im Plugin-Konfigurationsverzeichnis gespeichert. Bearbeiten speichert lokal; erst „anwenden“ ändert Herdr Projects Einstellungen.

## Grenzen

Das Plugin startet oder steuert CLIs nicht selbst. Agenten laufen über die interaktive Terminal-Integration von Herdr Projects in sichtbaren Split-Panes. ACP und Headless-Modi werden nicht verwendet. Bei interaktiven CLIs ohne Transcript-Leser erkennt Herdr Projects eindeutige Quota- und Providerfehler aus den letzten Terminalzeilen, nachdem Herdr das Pane als idle oder done meldet. Die jeweilige CLI muss diese Fehlermeldung sichtbar ausgeben.
