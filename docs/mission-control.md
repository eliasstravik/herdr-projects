# Mission Control

Mission Control bleibt als eigener Herdr-Arbeitsbereich offen. Links stehen die Projektordner unter `~/Developer`, auch wenn es dort noch keine Tasks gibt. Ein Klick wählt den Ordner für die Eingabe unten. Rechts läuft der Orchestrator als echte interaktive CLI; jedes delegierte Teilstück öffnet daneben ein eigenes gekacheltes Terminal im selben Tab.

## Einen Task abgeben

1. Mission Control über `herdr-control.open` oder `herdr-projects.open` öffnen. Die Control-Erweiterung öffnet es beim Herdr-Start automatisch. Erneutes Öffnen fokussiert die vorhandene Ansicht.
2. Links einen Projektordner wählen und unten den Auftrag schreiben. Entwürfe bleiben beim Wechsel zwischen Projekten erhalten.
3. **Task senden** anklicken oder Enter drücken. Der ausgewählte Ordner wird zusammen mit dem Auftrag gespeichert. Der Orchestrator verteilt die Arbeit mit Herdr Projects; der Mensch muss keine CLI einzeln öffnen.

Die globale CLI-Reihenfolge wird über **CLI-Reihenfolge** bearbeitet. Die erste CLI bestimmt den Orchestrator und den primären Worker; die restlichen sind die geordneten Fallbacks. Antigravity heißt `agy`; `antigravity` und `antigravity-cli` werden ebenfalls angenommen. Modelle sind optional als `cli/modell` möglich. Die Ansicht zeigt sowohl die gewählte erste CLI als auch die tatsächlich laufende CLI. Ein Prioritätswechsel wird nach dem Ende des laufenden Orchestrator-Turns umgesetzt.

## Delegation und Review

Der Orchestrator bleibt in derselben Sitzung sichtbar. Er legt Teilaufgaben mit dem gewählten Projektkontext an; Herdr Projects erstellt isolierte Git-Worktrees und startet die CLIs rechts daneben. Die linke Eingabe bleibt erhalten. Es entstehen keine zusätzlichen Agenten-Tabs. Fehler und Rückfragen erscheinen in den echten Terminal-Panes und im Task-Status.

Mit `review.by coordinator` prüft die bestehende Orchestrator-Sitzung die Ergebnisse. Der Workflow schickt ihr einen Review-Auftrag mit Arbeitsordner und eindeutigem Urteilspfad. Ihr Urteil benutzt denselben Befund- und Korrekturmechanismus wie der bisherige Reviewer: Fehler gehen automatisch zum Worker zurück. `review.by agent` behält den separaten Reviewer für die klassische Board-Nutzung. Mission Control setzt `review.when always` und `review.by coordinator`.

Provider- und Kontingentfehler werden bei ruhender CLI anhand eindeutiger Fehlerausgaben erkannt. Beim Orchestrator bleibt der fehlerhafte Terminal-Pane offen, der nächste installierte Eintrag übernimmt in einem neuen Split-Pane und bekommt die gespeicherte Vorgeschichte. Die ursprüngliche erste Priorität wird dabei nicht umgeschrieben. Workern steht der bestehende taskbezogene Provider-Fallback zur Verfügung. Ein Login- oder Berechtigungsdialog bleibt sichtbar und wartet auf den Menschen.

Die Projekt-AGENTS.md und die übergeordnete Developer/AGENTS.md gelten für die Arbeit. Der Git-Remote wird in der Ansicht angezeigt. Der bestehende Projects-Workflow übernimmt Commit, Prüfung, Push und Pull Request; ein Merge benötigt weiterhin eine Freigabe. Ein Ordner ohne Git-Remote wird ausdrücklich als lokaler Ordner angezeigt; dafür existiert noch kein GitHub-Synchronisationsziel.

## Installation aus diesem Feature-Branch

Dieser Branch benötigt seine eigenen Änderungen. `bin/run.sh` ruft deshalb `bin/source.sh` auf und ersetzt sie niemals durch das unveränderte Upstream-Release mit derselben Versionsnummer. Ein bereits geprüftes lokales Build wird benutzt; GitHub-Installationen teilen ansonsten ihren Cargo-Build unter `~/.cache/herdr-control-build`.

Für lokale Entwicklung:

```sh
npm run check mission_control
herdr plugin link /Users/family/Developer/herdr-projects
herdr plugin link /Users/family/Developer/herdr-projects/herdr-control
herdr server reload-config
```

Der Control-Plugin-Start und die Projects-Hooks müssen aus demselben Quellstand kommen. Die Quellarchive vor größeren Änderungen liegen lokal unter `Archiv/`.
