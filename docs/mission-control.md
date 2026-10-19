# Mission Control

Mission Control bleibt als eigener Herdr-Arbeitsbereich offen. Links stehen die Projektordner unter `~/Developer`, auch wenn es dort noch keine Tasks gibt. Ein Klick wählt den Ordner für die Eingabe unten. Die offenen Agenten-Tasks stehen ausschließlich in Herdrs nativer Agentenliste ganz links mit Task-ID, Titel und Status. Mission Control enthält keine zweite Taskliste und keine Kopie der Terminal-Ausgabe. Rechts läuft der Orchestrator als echte interaktive CLI; jedes delegierte Teilstück öffnet daneben ein eigenes gekacheltes Terminal im selben Tab.

## Einen Task abgeben

1. Mission Control über `herdr-control.open` oder `herdr-projects.open` öffnen. Die Control-Erweiterung öffnet es beim Herdr-Start automatisch. Erneutes Öffnen fokussiert die vorhandene Ansicht.
2. Links einen Projektordner wählen und unten den Auftrag schreiben. Entwürfe bleiben beim Wechsel zwischen Projekten erhalten.
3. **Task senden** anklicken oder Enter drücken. Der ausgewählte Ordner und seine eigenen Tasks stehen ausdrücklich in jeder übergebenen Nachricht. Statusfragen beziehen sich ausschließlich auf dieses Projekt. Der Orchestrator verteilt die Arbeit mit Herdr Projects; der Mensch muss keine CLI einzeln öffnen.

Die globale CLI-Reihenfolge wird über **CLI-Reihenfolge** bearbeitet. Die erste CLI bestimmt den Orchestrator. Worker nehmen die nächste verfügbare andere CLI aus derselben Liste. Auch bei Provider-Fallbacks bleibt die laufende Orchestrator-CLI für Worker ausgeschlossen; unterschiedliche Modelle derselben CLI zählen nicht als andere CLI. Die Liste braucht mindestens zwei unterschiedliche CLIs. Antigravity heißt `agy`; `antigravity` und `antigravity-cli` werden ebenfalls angenommen. Modelle sind optional als `cli/modell` möglich. Die Ansicht zeigt sowohl die gewählte erste CLI als auch die tatsächlich laufende CLI. Ein Prioritätswechsel wird nach dem Ende des laufenden Orchestrator-Turns umgesetzt.

## Delegation und Review

Der Orchestrator bleibt in derselben Sitzung sichtbar und übernimmt ausschließlich Koordination und Reviews. Jede Umsetzung delegiert er. Er legt Teilaufgaben mit dem gewählten Projektkontext an; Herdr Projects erstellt isolierte Git-Worktrees und startet die CLIs rechts daneben. Die linke Eingabe bleibt erhalten. Es entstehen keine zusätzlichen Agenten-Tabs. Fehler und Rückfragen erscheinen in den echten Terminal-Panes und im Task-Status. Ein Klick auf einen Task in Herdrs nativer Agentenliste fokussiert dessen Terminal. Ein gestoppter Task mit vorhandenem Terminal bleibt dort als wartender Eintrag sichtbar; der Workflow zählt diesen Anzeigeeintrag niemals als laufende CLI. Noch nicht gestartete Tasks ohne Terminal stehen weiterhin im klassischen Projects-Board.

Mit `review.by coordinator` prüft die bestehende Orchestrator-Sitzung die Ergebnisse. Der Workflow schickt ihr einen Review-Auftrag mit Arbeitsordner und eindeutigem Urteilspfad. Ihr Urteil benutzt denselben Befund- und Korrekturmechanismus wie der bisherige Reviewer: Fehler gehen automatisch zum Worker zurück. `review.by agent` behält den separaten Reviewer für die klassische Board-Nutzung. Mission Control setzt `review.when always` und `review.by coordinator`.

Provider- und Kontingentfehler werden bei ruhender CLI anhand eindeutiger Fehlerausgaben erkannt. Beim Orchestrator übernimmt der nächste installierte Eintrag in einem neuen Split-Pane und bekommt die gespeicherte Vorgeschichte. Sobald der neue Orchestrator bereit ist, wird die Ausgabe des alten im Datenordner unter `coordinator/<session>/handover-*.terminal.txt` gesichert. Danach schließt ausschließlich dessen ruhender Pane. Ein arbeitender Vorgänger wird nicht beendet. Damit bleibt ein aktueller Orchestrator sichtbar, statt mehrere alte Sitzungen zu sammeln. Die ursprüngliche erste Priorität wird dabei nicht umgeschrieben. Workern steht der bestehende taskbezogene Provider-Fallback zur Verfügung. Ein Login- oder Berechtigungsdialog bleibt sichtbar und wartet auf den Menschen.

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

## Ruhige Terminal-Anzeige

Automatische Status-, Fehler- und Ausgabeabfragen lesen ausschließlich das sichtbare Terminalbild. Sie sammeln keinen internen CLI-Gesprächsverlauf und schicken keine Scroll-Tasten. Das gilt auch bei einer als bereit oder fertig gemeldeten CLI, damit Cline und andere Terminals ihre Scrollposition behalten. Gesicherte Terminal-Protokolle enthalten entsprechend den sichtbaren Ausschnitt.
