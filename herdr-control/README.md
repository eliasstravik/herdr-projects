# Herdr Control — Mission Control

Dauerhaft geöffnete Projektansicht für Herdr: links die Ordner unter `~/Developer`, unten ein Task-Eingabefeld, rechts der ausgewählte Orchestrator und seine delegierten CLIs als sichtbare echte Split-Panes im selben Tab.

## Bedienung

1. Links einen Projektordner wählen.
2. Unten einen Task eingeben und **Task senden** wählen.
3. Der Orchestrator teilt die Arbeit auf, öffnet die CLIs rechts und prüft ihre Ergebnisse in seiner laufenden Sitzung. Fortschritt, Fehler und Rückfragen bleiben sichtbar. Task-Zeilen öffnen per Klick das jeweilige Terminal; der Rückmeldungsbereich zeigt die Live-Ausgabe und öffnet per Klick den Orchestrator.

**CLI-Reihenfolge** legt eine globale Reihenfolge für Orchestrator und Worker fest. Die erste CLI orchestriert und reviewed. Umsetzungen übernehmen Worker mit der nächsten verfügbaren anderen CLI; die laufende Orchestrator-CLI ist auch bei Worker-Fallbacks ausgeschlossen. Mindestens zwei unterschiedliche CLIs sind erforderlich. Unterstützt: Freebuff (`freebuff`), CodeBuddy (`codebuddy`), Claude Code (`claude`), Codex (`codex`), Antigravity (`agy`), Cursor (`cursor`), OpenCode (`opencode`), Kiro (`kiro`), Cline (`cline`), Qoder (`qodercli`) und Grok (`grok`). Modelle sind optional als `cli/modell` möglich. Antigravity kann auch als `antigravity-cli` oder `antigravity` eingegeben werden.

Das Plugin öffnet Mission Control beim Herdr-Start automatisch. Die Aktionen `herdr-control.open` und `herdr-projects.open` fokussieren dieselbe Ansicht. Die Projects-Tastenbelegung öffnet sie ebenfalls. Bestehende Agenten bleiben beim Wechsel der Ansicht sichtbar und laufen weiter. Ein Prioritätswechsel wartet einen laufenden Orchestrator-Turn ab.

## Installation

Beide Plugins aus demselben Feature-Branch installieren:

```sh
herdr plugin install ysamjo/herdr-projects --ref feature/interactive-cli-pool
herdr plugin install ysamjo/herdr-projects/herdr-control --ref feature/interactive-cli-pool
```

Dieser Feature-Branch baut seinen eigenen Quellstand mit Rust. Ein Upstream-Release darf die Änderungen nicht ersetzen. Weitere Einzelheiten und lokale Entwicklungsbefehle: [Mission Control](../docs/mission-control.md).

ACP und Headless-Modi werden für die CLIs nicht verwendet. Projekt-AGENTS.md und Git-Remote bestimmen die Arbeit und den späteren GitHub-Push. Ordner ohne Remote werden als lokal angezeigt.
