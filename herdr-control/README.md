# Herdr Control — Mission Control

Dauerhaft geöffnete Projektansicht für Herdr: für jeden direkten Unterordner von `~/Developer` gibt es einen eigenen Herdr-Space. Jeder Space hat sein eigenes Task-Eingabefeld und seinen eigenen sichtbaren Orchestrator. Die Ordnerliste entfällt; die Herdr-Space-Leiste übernimmt die Projektauswahl. Worker-CLIs werden als echte interaktive Split-Panes im selben Space geöffnet. Ein Projektwechsel überschreibt keine laufende Sitzung.

## Bedienung

1. Den Projekt-Space in Herdr auswählen.
2. Im eigenen Taskfeld den Auftrag eingeben und **Task senden** wählen.
3. Der Orchestrator teilt die Arbeit auf, öffnet Worker-Panes im selben Space und prüft ihre Ergebnisse in seiner laufenden Sitzung. Fortschritt, Fehler und Rückfragen bleiben sichtbar. Offene Agenten-Tasks stehen in Herdrs Agentenliste ganz links mit Task-ID, Titel und Status; ein Klick fokussiert das Terminal.

**CLI-Reihenfolge** legt eine globale Reihenfolge für Orchestrator und Worker fest. Die erste CLI orchestriert und reviewed. Umsetzungen übernehmen Worker mit der nächsten verfügbaren anderen CLI; die laufende Orchestrator-CLI ist auch bei Worker-Fallbacks ausgeschlossen. Mindestens zwei unterschiedliche CLIs sind erforderlich. Unterstützt: Freebuff (`freebuff`), CodeBuddy (`codebuddy`), Claude Code (`claude`), Codex (`codex`), Antigravity (`agy`), Cursor (`cursor`), OpenCode (`opencode`), Kiro (`kiro`), Cline (`cline`), Qoder (`qodercli`) und Grok (`grok`). Modelle sind optional als `cli/modell` möglich. Antigravity kann auch als `antigravity-cli` oder `antigravity` eingegeben werden.

Das Plugin legt beim Öffnen für jeden direkten Projektordner unter `~/Developer` einen Space mit Taskfeld an. `herdr-control.open` und `herdr-projects.open` öffnen beziehungsweise fokussieren diese Spaces; Herdrs Space-Leiste wechselt zwischen Projekten. Coordinator und Worker werden erst beim ersten Task gestartet und bleiben beim Wechsel erhalten. Ein Prioritätswechsel wartet einen laufenden Orchestrator-Turn ab.

## Installation

Beide Plugins aus demselben Feature-Branch installieren:

```sh
herdr plugin install ysamjo/herdr-projects --ref feature/interactive-cli-pool
herdr plugin install ysamjo/herdr-projects/herdr-control --ref feature/interactive-cli-pool
```

Dieser Feature-Branch baut seinen eigenen Quellstand mit Rust. Ein Upstream-Release darf die Änderungen nicht ersetzen. Weitere Einzelheiten und lokale Entwicklungsbefehle: [Mission Control](../docs/mission-control.md).

ACP und Headless-Modi werden für die CLIs nicht verwendet. Projekt-AGENTS.md und Git-Remote bestimmen die Arbeit und den späteren GitHub-Push. Ordner ohne Remote bleiben lokale Spaces ohne eingerichtetes GitHub-Ziel.
