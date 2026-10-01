# The steps of smoke.yml, one function each, for Windows, macOS and Linux.
# A step dot-sources this file and calls its function, so the workflow keeps
# one step (and one timeout) per check while the checks live here.
#
# Most of it is the same everywhere. What differs is marked "Windows:" or
# "Unix:" where it happens:
# - installing herdr (install.ps1 or install.sh) and starting its server;
# - the shell in herdr's panes (PowerShell, or bash/zsh), so the lines typed;
# - the command link (a .cmd shim, or a symbolic link), the skill link (a
#   junction, or a symbolic link) and CLAUDE.md (an @AGENTS.md pointer file,
#   or a symbolic link);
# - the routine with a shell (pwsh, or sh);
# - copying logs and stopping processes.
. (Join-Path $PSScriptRoot "helpers.ps1")

function Initialize-Smoke {
    $t = $env:RUNNER_TEMP
    $dirs = [ordered]@{
        HERDR_PROJECTS_ROOT = Join-Path $t "hp-root"
        STUB_LOG_DIR        = Join-Path $t "stub-logs"
        SMOKE_ARTIFACTS     = Join-Path $t "smoke-artifacts"
        HERDR_INSTALL_DIR   = Join-Path $t "herdr-bin"
        STUB_DIR            = Join-Path $t "stub"
    }
    foreach ($name in $dirs.Keys) {
        New-Item -ItemType Directory -Force $dirs[$name] | Out-Null
        "$name=$($dirs[$name])" | Add-Content $env:GITHUB_ENV
    }
    "HP_EXE=$(Join-Path $env:GITHUB_WORKSPACE "target" "release" "herdr-projects$Exe")" | Add-Content $env:GITHUB_ENV
    # Where the plugin puts its herdr-projects command.
    $bin = Join-Path $HOME ".local" "bin"
    # Searched first to last: the stub must win over any real claude.
    foreach ($dir in @($dirs.STUB_DIR, $dirs.HERDR_INSTALL_DIR, $bin)) { $dir | Add-Content $env:GITHUB_PATH }
    if ($IsWindows) {
        # Windows: herdr's panes take PATH from the registry (machine and
        # user), not from the server's environment (run 36843818657 showed a
        # pane without the folders above). Installers put claude, herdr and
        # the shim folder on the user PATH, so do the same here. Unix panes
        # inherit the server's environment.
        $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
        $front = @($dirs.STUB_DIR, $dirs.HERDR_INSTALL_DIR, $bin) -join ";"
        [Environment]::SetEnvironmentVariable("Path", "$front;$userPath", "User")
    }
    # Rows for the job summary, which Write-Summary makes into one table.
    $results = Join-Path $t "smoke-results.md"
    New-Item -ItemType File -Force $results | Out-Null
    "SMOKE_RESULTS=$results" | Add-Content $env:GITHUB_ENV
}

# The installers herdr documents; both honour HERDR_INSTALL_DIR.
function Install-Herdr {
    if ($IsWindows) {
        irm https://herdr.dev/install.ps1 | iex
        $global:LASTEXITCODE = 0
    } else {
        Invoke-Checked sh @("-c", "curl -fsSL https://herdr.dev/install.sh | sh") | Out-Null
    }
    Invoke-Checked (Join-Path $env:HERDR_INSTALL_DIR "herdr$Exe") @("--version") | Out-Null
}

function Build-Stub {
    $stub = Join-Path $env:STUB_DIR "claude$Exe"
    Invoke-Checked rustc @("--edition", "2021", "-O", (Join-Path $PSScriptRoot "claude_stub.rs"), "-o", $stub) | Out-Null
    Invoke-Checked $stub @("--version") | Out-Null
    $found = (Get-Command claude -CommandType Application | Select-Object -First 1).Source
    if ($found -ne $stub) { throw "claude resolves to $found, not the stub" }
}

function Connect-Plugin {
    if ($IsWindows) {
        # Windows: "windows" stays out of the manifest's top-level `platforms`
        # until this smoke test passes reliably (decided 2026-10-01). Until
        # then herdr skips the plugin's actions, panes and startup hook on
        # Windows, so this job adds it in its own checkout only. Remove this
        # patch once the manifest lists "windows".
        $manifest = Join-Path $env:GITHUB_WORKSPACE "herdr-plugin.toml"
        $lines = [System.IO.File]::ReadAllLines($manifest)
        $patched = $false
        for ($i = 0; $i -lt $lines.Length; $i++) {
            if ($lines[$i].StartsWith("[[")) { break }  # top level only, not [[build]] entries
            if ($lines[$i] -match '^platforms\s*=\s*\[(.*)\]\s*$') {
                if ($Matches[1] -notmatch '"windows"') { $lines[$i] = "platforms = [$($Matches[1]), `"windows`"]" }
                $patched = $true
                break
            }
        }
        if (-not $patched) { throw "no top-level platforms in herdr-plugin.toml" }
        [System.IO.File]::WriteAllLines($manifest, $lines)
    }
    Select-String -Path (Join-Path $env:GITHUB_WORKSPACE "herdr-plugin.toml") -Pattern '^platforms' | Select-Object -First 1 | ForEach-Object { Write-Host $_.Line }
    # `plugin link`, not `plugin install`: install hits herdr#4179 on
    # Windows, and link tests this checkout everywhere. Linking before the
    # server starts lets herdr run the plugin's startup hook at server start,
    # as for a user.
    Invoke-Checked herdr @("plugin", "link", $env:GITHUB_WORKSPACE) | Out-Null
    $list = Invoke-Checked herdr @("plugin", "list", "--json")
    if ($list -notmatch 'herdr-projects') { throw "plugin list does not show herdr-projects" }
}

function Test-ProjectCreated {
    # Before the server starts, so the startup hook finds a project and
    # starts the ticker.
    Invoke-Check "project created" {
        Hp new smoke --goal "Smoke test" | Out-Null
        $dir = Join-Path $env:HERDR_PROJECTS_ROOT "smoke"
        foreach ($file in @("PROJECT.md", "AGENTS.md", "CLAUDE.md")) {
            if (-not (Test-Path (Join-Path $dir $file))) { throw "$file missing in $dir" }
        }
        "in $dir"
    }
}

function Start-HerdrServer {
    $herdr = Join-Path $env:HERDR_INSTALL_DIR "herdr$Exe"
    if ($IsWindows) {
        # Windows: herdr's own CI pattern (scripts/windows_smoke_conpty_path.ps1).
        $server = Start-Process -FilePath $herdr -ArgumentList "server" -PassThru -WindowStyle Hidden
        $serverPid = $server.Id
    } else {
        # Unix: herdr's own CI pattern (scripts/smoke_live_handoff_sessions.sh),
        # `herdr server` in the background, with its output kept for the logs.
        $out = Join-Path $env:SMOKE_ARTIFACTS "herdr-server.out"
        $serverPid = (Invoke-Checked sh @("-c", 'nohup "$1" server >"$2" 2>&1 </dev/null & echo $!', "sh", $herdr, $out)).Trim()
    }
    "HERDR_SERVER_PID=$serverPid" | Add-Content $env:GITHUB_ENV
    Wait-Until "herdr server running" 30 {
        $status = Invoke-Quiet herdr @("status", "server")
        $status.Code -eq 0 -and $status.Text -match "status: running"
    } -IntervalMs 250 | Out-Null
    Invoke-Checked herdr @("status", "server") | Out-Null
}

function Test-PaneFindsStub {
    # Agents start in pane shells, which get herdr's environment: the stub
    # must resolve there, not only in this step.
    Invoke-Check "pane resolves claude to the stub" {
        $created = (Invoke-Checked herdr @("workspace", "create", "--cwd", $env:RUNNER_TEMP, "--label", "terminal")) | ConvertFrom-Json
        $term = $created.result.root_pane.pane_id
        "TERMINAL_PANE=$term" | Add-Content $env:GITHUB_ENV
        if ($IsWindows) {
            # Windows: PowerShell.
            Wait-Until "a shell prompt in $term" 30 { (Read-Pane $term) -match "PS |>" } | Out-Null
            Send-PaneLine $term 'Write-Output "PATH=$env:PATH"; Write-Output "CLAUDE=$((Get-Command claude -ErrorAction SilentlyContinue).Source)"; Write-Output "LOGDIR=$env:STUB_LOG_DIR"'
        } else {
            # Unix: the runner's $SHELL (bash). The split names keep the
            # typed line from matching the answer below.
            Wait-Until "a shell prompt in $term" 30 { (Read-Pane $term) -match '(?m)[$#%>]\s*$' } | Out-Null
            Send-PaneLine $term 'echo "PA""TH=$PATH"; echo "CLAU""DE=$(command -v claude)"; echo "LOG""DIR=$STUB_LOG_DIR"'
        }
        $screen = Wait-Until "the pane printed CLAUDE=" 30 { $t = Read-Pane $term; if ($t -match "(?m)^CLAUDE=") { $t } }
        Write-Host $screen
        $claude = [regex]::Match($screen, "(?m)^CLAUDE=(.*)$").Groups[1].Value.Trim()
        if ($claude -ne (Join-Path $env:STUB_DIR "claude$Exe")) { throw "claude in a pane resolves to '$claude'" }
        $claude
    }
}

function Test-StartupHook {
    Invoke-Check "startup hook starts the ticker" {
        $status = Wait-Until "ticker running" 45 {
            $s = Invoke-Quiet $env:HP_EXE @("ticker", "status")
            if ($s.Text -match "ticker: running") { $s.Text }
        }
        Write-Host $status
        if ($status -notmatch "pid:\s+(\d+)") { throw "no pid in ticker status" }
        "TICKER_PID=$($Matches[1])" | Add-Content $env:GITHUB_ENV
        "pid $($Matches[1])"
    }
    # herdr records the startup command as finished only once its output
    # pipes close: a ticker that inherited them kept it `running`.
    Invoke-Check "startup hook finishes" {
        $entry = Wait-Until "the startup command no longer running" 30 {
            $logs = ((Invoke-Quiet herdr @("plugin", "logs", "--plugin", "herdr-projects")).Text | ConvertFrom-Json).result.logs
            $logs | Where-Object { $_.event -eq "startup" -and $_.status -ne "running" } | Select-Object -First 1
        }
        if ($entry.status -ne "succeeded") { throw "startup command $($entry.status): $($entry | ConvertTo-Json -Compress)" }
        $entry.status
    }
}

function Test-TickerSurvives {
    # The startup hook is a short-lived child of herdr; the ticker it starts
    # is detached (a new session on Unix, DETACHED_PROCESS on Windows). herdr
    # may end what is left when a plugin command ends (a Job object on
    # Windows, the process group on Unix): the open question from t-0003.
    Invoke-Check "ticker survives its starting command" {
        if (-not $env:TICKER_PID) { throw "no ticker pid recorded" }
        Start-Sleep -Seconds 20
        $logs = Invoke-Quiet herdr @("plugin", "logs", "--plugin", "herdr-projects")
        Write-Host $logs.Text
        if (-not (Get-Process -Id $env:TICKER_PID -ErrorAction SilentlyContinue)) { throw "ticker process $env:TICKER_PID is gone" }
        $s = Invoke-Quiet $env:HP_EXE @("ticker", "status")
        if ($s.Text -notmatch "pid:\s+$env:TICKER_PID\b") { throw "ticker status changed: $($s.Text)" }
        "pid $env:TICKER_PID alive after 20 s"
    }
}

function Test-Configure {
    Invoke-Check "configure" {
        Hp configure --clients claude | Out-Null
        $settings = Get-Content -Raw (Join-Path $HOME ".claude" "settings.json")
        Write-Host $settings
        if ($settings -notmatch 'hook --agent claude') { throw "no hook in .claude/settings.json" }
        "hooks in .claude/settings.json"
    }
}

function Test-Coordinator {
    Invoke-Check "coordinator opens" {
        Hp open smoke | Out-Null
        $record = Get-Content -Raw (Join-Path $env:HERDR_PROJECTS_ROOT "smoke" ".state" "coordinator.json") | ConvertFrom-Json
        if (-not $record.pane_id) { throw "coordinator.json has no pane" }
        "COORDINATOR_PANE=$($record.pane_id)" | Add-Content $env:GITHUB_ENV
        $agent = Wait-Until "coordinator agent idle" 60 {
            $list = (Invoke-Quiet herdr @("agent", "list")).Text | ConvertFrom-Json
            $list.result.agents | Where-Object { $_.pane_id -eq $record.pane_id -and $_.agent_status -eq "idle" }
        }
        Write-Host ($agent | ConvertTo-Json -Depth 5)
        "pane $($record.pane_id), agent $($agent.name)"
    }
}

function Test-ThreadAndBrief {
    Invoke-Check "thread starts" {
        $task = Join-Path $env:RUNNER_TEMP "task.md"
        "Smoke task: reply with OK." | Set-Content -Encoding utf8 $task
        Hp thread start smoke --title "Smoke thread" --task-file $task --kind tab | Out-Null
        $record = Wait-Until "thread t-0001 placed with a pane" 30 {
            $r = Get-ThreadRecord smoke t-0001
            if ($r -and $r.pane_id -and $r.status -eq "open") { $r }
        }
        "THREAD_PANE=$($record.pane_id)" | Add-Content $env:GITHUB_ENV
        $agent = Wait-Until "thread agent running" 60 {
            $list = (Invoke-Quiet herdr @("agent", "list")).Text | ConvertFrom-Json
            $list.result.agents | Where-Object { $_.pane_id -eq $record.pane_id -and $_.agent -eq "claude" }
        }
        "pane $($record.pane_id), agent $($agent.name)"
    }
    # herdr#4529: on Windows the first prompt to a new agent is often typed
    # but not submitted. The stub logs every line it receives.
    Invoke-Check "brief arrives" {
        $pane = (Get-ThreadRecord smoke t-0001).pane_id
        Wait-Until "the stub received the brief line" 60 {
            (Get-StubLog $pane) -match "prompt: Read \.herdr-project/smoke-t-0001/brief\.md"
        } -IntervalMs 2000 | Out-Null
        $record = Wait-Until "the record says delivered" 30 {
            $r = Get-ThreadRecord smoke t-0001
            if ($r -and -not $r.prompt_pending) { $r }
        }
        $brief = Join-Path $record.thread_dir "brief.md"
        if (-not (Test-Path -LiteralPath $brief)) { throw "no brief at $brief" }
        "after $($record.brief_attempts) attempt(s)"
    }
}

function Test-SameTicker {
    Invoke-Check "ticker survives across steps" {
        if (-not $env:TICKER_PID) { throw "no ticker pid recorded" }
        if (-not (Get-Process -Id $env:TICKER_PID -ErrorAction SilentlyContinue)) { throw "ticker process $env:TICKER_PID is gone" }
        $s = Invoke-Checked $env:HP_EXE @("ticker", "status")
        if ($s -notmatch "pid:\s+$env:TICKER_PID\b") { throw "another ticker runs now: $s" }
        "pid $env:TICKER_PID"
    }
}

function Test-Links {
    Invoke-Check "herdr-projects command" {
        if ($IsWindows) {
            # Windows: a .cmd shim that forwards every argument.
            $shim = Join-Path $HOME ".local" "bin" "herdr-projects.cmd"
            if (-not (Test-Path -LiteralPath $shim)) { throw "no $shim" }
            $text = (Get-Content -Raw -LiteralPath $shim).TrimEnd()
            $expected = "@`"$env:HP_EXE`" %*"
            if ($text -ne $expected) { throw "shim is '$text', expected '$expected'" }
            $version = Invoke-Checked cmd @("/d", "/c", "herdr-projects", "--version")
        } else {
            # Unix: a symbolic link to the binary.
            $link = Join-Path $HOME ".local" "bin" "herdr-projects"
            $item = Get-Item -LiteralPath $link -Force
            if ($item.LinkType -ne "SymbolicLink") { throw "$link is '$($item.LinkType)', not a symbolic link" }
            $target = [System.IO.File]::ResolveLinkTarget($link, $true).FullName
            if ($target -ne $env:HP_EXE) { throw "$link resolves to $target, not $env:HP_EXE" }
            $version = Invoke-Checked herdr-projects @("--version")
        }
        if ($version -notmatch "herdr-projects \d") { throw "the command ran: $version" }
        $version
    }
    Invoke-Check "skill link" {
        $link = Join-Path $HOME ".claude" "skills" "autoproject"
        $item = Get-Item -LiteralPath $link -Force
        # Windows: a junction, which needs no special rights. Unix: a symbolic link.
        $kind = if ($IsWindows) { "Junction" } else { "SymbolicLink" }
        if ($item.LinkType -ne $kind) { throw "$link is '$($item.LinkType)', not a $kind" }
        $target = [string]$item.Target
        if ($target -ne (Join-Path $env:GITHUB_WORKSPACE "skill" "autoproject")) { throw "$link points to $target" }
        if (-not (Test-Path -LiteralPath (Join-Path $link "SKILL.md"))) { throw "SKILL.md not readable through the link" }
        "$kind -> $target"
    }
    Invoke-Check "CLAUDE.md reads AGENTS.md" {
        $claude = Join-Path $env:HERDR_PROJECTS_ROOT "smoke" "CLAUDE.md"
        $item = Get-Item -LiteralPath $claude -Force
        if ($IsWindows) {
            # Windows: a plain file importing AGENTS.md (links need rights).
            if ($item.LinkType) { throw "CLAUDE.md is a $($item.LinkType), expected a plain file" }
            $text = Get-Content -Raw -LiteralPath $claude
            if ($text -ne "@AGENTS.md`n") { throw "CLAUDE.md holds '$text'" }
            "@AGENTS.md"
        } else {
            # Unix: a relative symbolic link.
            if ($item.LinkType -ne "SymbolicLink" -or [string]$item.Target -ne "AGENTS.md") { throw "CLAUDE.md is '$($item.LinkType)' to '$($item.Target)'" }
            $agents = Get-Content -Raw -LiteralPath (Join-Path $env:HERDR_PROJECTS_ROOT "smoke" "AGENTS.md")
            if ((Get-Content -Raw -LiteralPath $claude) -ne $agents) { throw "CLAUDE.md does not read as AGENTS.md" }
            "-> AGENTS.md"
        }
    }
}

function Test-HooksAndReport {
    $pane = $env:THREAD_PANE
    $log = Get-StubLog $pane
    Write-Host $log
    # The stub runs the configured command as Claude Code does: /bin/sh on
    # Unix; Git Bash on Windows when it is installed (it is on the runner).
    Invoke-Check "SessionStart hook" {
        $line = ($log -split "`n") | Where-Object { $_ -match "hook \(\w+\) SessionStart" } | Select-Object -First 1
        if (-not $line) { throw "the stub never ran the SessionStart hook" }
        if ($line -notmatch "exit=Some\(0\)" -or $line -notmatch "report --percent") { throw $line }
        ($line -split "stdout=")[0]
    }
    Invoke-Check "UserPromptSubmit hook" {
        $line = ($log -split "`n") | Where-Object { $_ -match "hook \(\w+\) UserPromptSubmit" } | Select-Object -First 1
        if (-not $line) { throw "the stub never ran the UserPromptSubmit hook" }
        if ($line -notmatch "exit=Some\(0\)" -or $line -notmatch "Before task tools") { throw $line }
        "answered"
    }
    Invoke-Check "report through the herdr-projects command" {
        $line = ($log -split "`n") | Where-Object { $_ -match "report via command" } | Select-Object -First 1
        if ($line -notmatch "exit=Some\(0\)") { throw "$line" }
        # Records are keyed by session socket, which `progress` reads from
        # HERDR_SOCKET_PATH, set only inside a pane.
        $env:HERDR_SOCKET_PATH = (Get-Content -Raw (Join-Path $env:HERDR_PROJECTS_ROOT "smoke" ".state" "coordinator.json") | ConvertFrom-Json).socket
        $progress = Invoke-Checked $env:HP_EXE @("progress", "--pane", $pane)
        Remove-Item Env:HERDR_SOCKET_PATH
        if ($progress -notmatch "Stub working") { throw "progress record: $progress" }
        "recorded"
    }
}

function Test-Routines {
    $dir = Join-Path $env:HERDR_PROJECTS_ROOT "smoke" "routines"
    # No shell: the default on Windows, and opt-out-able on Unix. A bare
    # command name found through PATH (and PATHEXT on Windows: the .cmd shim).
    @(
        '+++',
        'schedule = "every 1m"',
        'shell = "none"',
        "command = 'herdr-projects --version'",
        '+++',
        'Smoke routine without a shell.'
    ) -join "`n" | Set-Content -NoNewline -Encoding utf8 (Join-Path $dir "smoke-none.md")
    if ($IsWindows) {
        # Windows: pwsh.
        $shellName = "pwsh"
        $command = "command = 'Write-Output (""SMOKE-SHELL-"" + (40 + 2))'"
    } else {
        # Unix: sh.
        $shellName = "sh"
        $command = "command = 'echo ""SMOKE-SHELL-`$((40 + 2))""'"
    }
    @(
        '+++',
        'schedule = "every 1m"',
        "shell = `"$shellName`"",
        $command,
        '+++',
        "Smoke routine with $shellName."
    ) -join "`n" | Set-Content -NoNewline -Encoding utf8 (Join-Path $dir "smoke-shell.md")
    $names = @("smoke-none", "smoke-shell")
    if ($IsWindows) {
        # Windows: the ticker runs with no console of its own to show. A
        # console program it starts without CREATE_NO_WINDOW gets a new,
        # visible console window, which flashes up on every run. This routine
        # reports the console window it runs in.
        $probe = 'Add-Type -Namespace Smoke -Name Console -MemberDefinition ''[DllImport("kernel32.dll")] public static extern System.IntPtr GetConsoleWindow(); [DllImport("user32.dll")] public static extern bool IsWindowVisible(System.IntPtr window);''; $window = [Smoke.Console]::GetConsoleWindow(); $state = if ($window -eq [System.IntPtr]::Zero) { "none" } elseif ([Smoke.Console]::IsWindowVisible($window)) { "visible" } else { "hidden" }; Write-Output "SMOKE-CONSOLE-$state"'
        @(
            '+++',
            'schedule = "every 1m"',
            'shell = "pwsh"',
            "command = '''$probe'''",
            '+++',
            'Smoke routine that reports its console window.'
        ) -join "`n" | Set-Content -NoNewline -Encoding utf8 (Join-Path $dir "smoke-console.md")
        $names += "smoke-console"
    }

    # `safety set` and `routine approve` refuse without a terminal, so they
    # are typed into a herdr pane, as a person would.
    $term = $env:TERMINAL_PANE
    Invoke-Check "routine approval in a terminal" {
        Send-PaneLine $term "$env:HP_EXE safety set --global routine_commands on"
        Wait-Until "the y/N question" 30 { (Read-Pane $term) -match "\[y/N\]" } | Out-Null
        Send-PaneLine $term "y"
        Wait-Until "routine_commands on" 30 { (Read-Pane $term) -match "routine_commands = on" } | Out-Null
        foreach ($name in $names) {
            $before = ([regex]::Matches((Read-Pane $term), "Type the routine's name")).Count
            Send-PaneLine $term "$env:HP_EXE routine approve smoke $name"
            Wait-Until "the approve question for $name" 30 { ([regex]::Matches((Read-Pane $term), "Type the routine's name")).Count -gt $before } | Out-Null
            Send-PaneLine $term $name
            Wait-Until "$name approved" 30 { (Read-Pane $term) -match "approved ``$name``" } | Out-Null
        }
        $list = Invoke-Checked $env:HP_EXE @("routine", "list", "smoke")
        if (([regex]::Matches($list, "command: approved")).Count -lt $names.Count) { throw "routine list: $list" }
        "$($names.Count) approved"
    }
    # The ticker runs a due routine on its 15 s tick; the first run of an
    # `every 1m` routine is one minute after the ticker first sees it, so this
    # wait is the one that exceeds a minute. Both routines fall due on the
    # same tick.
    $inbox = Join-Path $env:HERDR_PROJECTS_ROOT "smoke" "inbox"
    function Find-Item([string] $Pattern) {
        Get-ChildItem -LiteralPath $inbox -Filter *.md -ErrorAction SilentlyContinue |
            Where-Object { (Get-Content -Raw -LiteralPath $_.FullName) -match $Pattern } | Select-Object -First 1
    }
    Invoke-Check "routine with no shell" {
        $item = Wait-Until "an inbox item with the version" 100 { Find-Item "herdr-projects \d+\.\d+" } -IntervalMs 5000
        Get-Content -Raw -LiteralPath $item.FullName | Write-Host
        $item.Name
    }
    Invoke-Check "routine with shell = $shellName" {
        $item = Wait-Until "an inbox item with SMOKE-SHELL-42" 30 { Find-Item "SMOKE-SHELL-42" } -IntervalMs 5000
        Get-Content -Raw -LiteralPath $item.FullName | Write-Host
        $item.Name
    }
    if ($IsWindows) {
        Invoke-Check "routine runs without a visible console window" {
            $item = Wait-Until "an inbox item with SMOKE-CONSOLE-" 30 { Find-Item "SMOKE-CONSOLE-" } -IntervalMs 5000
            $text = Get-Content -Raw -LiteralPath $item.FullName
            Write-Host $text
            $state = [regex]::Match($text, "SMOKE-CONSOLE-\w+").Value
            if ($state -notin @("SMOKE-CONSOLE-none", "SMOKE-CONSOLE-hidden")) { throw "the routine ran in a visible console window: $state" }
            $state
        }
    }
}

function Test-Doctor {
    Invoke-Check "doctor" {
        $out = Invoke-Quiet $env:HP_EXE @("doctor")
        Write-Host $out.Text
        if ($out.Code -ne 0) { throw "doctor exited $($out.Code): $((($out.Text -split "`n") | Where-Object { $_ -match 'FAIL|fail|✗|missing' }) -join '; ')" }
        "healthy"
    }
}

function Save-SmokeLogs {
    $ErrorActionPreference = "Continue"
    $out = $env:SMOKE_ARTIFACTS
    New-Item -ItemType Directory -Force $out | Out-Null
    function Save([string] $Name, [string[]] $Arguments) {
        (Invoke-Quiet $Arguments[0] $Arguments[1..($Arguments.Length - 1)]).Text | Set-Content -Encoding utf8 (Join-Path $out $Name)
    }
    Save "herdr-version.txt" @("herdr", "--version")
    Save "herdr-plugin-list.json" @("herdr", "plugin", "list", "--json")
    Save "herdr-plugin-logs.txt" @("herdr", "plugin", "logs", "--plugin", "herdr-projects", "--limit", "200")
    Save "herdr-agent-list.json" @("herdr", "agent", "list")
    Save "herdr-pane-list.json" @("herdr", "pane", "list")
    Save "ticker-status.txt" @($env:HP_EXE, "ticker", "status")
    Save "doctor.txt" @($env:HP_EXE, "doctor")
    Save "thread-list.json" @($env:HP_EXE, "thread", "list", "smoke", "--json")
    $panes = ((Invoke-Quiet herdr @("pane", "list")).Text | ConvertFrom-Json).result.panes
    foreach ($pane in $panes) {
        Save "pane-$($pane.pane_id -replace ':', '-').txt" @("herdr", "pane", "read", $pane.pane_id, "--source", "recent-unwrapped", "--lines", "200", "--format", "text")
    }
    foreach ($pair in @(
            @((Join-Path $ConfigHome "herdr"), "herdr-config"),
            @((Join-Path $ConfigHome "herdr-projects"), "herdr-projects-config"),
            @($env:HERDR_PROJECTS_ROOT, "projects-root"),
            @($env:STUB_LOG_DIR, "stub-logs"),
            @((Join-Path $HOME ".claude"), "claude-config"))) {
        if (Test-Path -LiteralPath $pair[0]) {
            $dest = Join-Path $out $pair[1]
            if ($IsWindows) {
                # Windows: robocopy copies what it can read and skips junction loops.
                robocopy $pair[0] $dest /E /XJ /R:0 /W:0 /NFL /NDL /NJH /NJS | Out-Null
            } else {
                # Unix: links stay links; sockets and the like are skipped with a warning.
                cp -RP $pair[0] $dest 2>&1 | Write-Host
            }
        }
    }
    if (-not $IsWindows) {
        # Unix: upload-artifact follows links and warns ENTRYNOTSUPPORTED for
        # anything that is not a regular file or folder at the end: herdr's
        # sockets (GNU cp copies them), and on macOS `herdr.sock.agent`, a link
        # to launchd's ssh-agent socket. Leave those out, and name them, with
        # their type, in the log and in left-out.txt. Links to files and
        # folders (CLAUDE.md, the skill) zip fine and stay. No `find -L`: rm
        # must never reach through a link into the checkout.
        # `test -f` and `test -d` follow links, as the zip does.
        $odd = @(& find $out '!' -type f '!' -type d | Where-Object {
                & sh -c 'test -f "$1" || test -d "$1"' sh $_
                $LASTEXITCODE -ne 0
            })
        $listing = foreach ($path in $odd) { & ls -ld $path }
        $listing | Set-Content -Encoding utf8 (Join-Path $out "left-out.txt")
        foreach ($line in $listing) { Write-Host "left out of the artifact: $line" }
        foreach ($path in $odd) { & rm -f -- $path }
    }
    $tickerLog = Join-Path $env:HERDR_PROJECTS_ROOT ".ticker.log"
    if (Test-Path -LiteralPath $tickerLog) {
        Write-Host "--- last 60 lines of .ticker.log"
        Get-Content -LiteralPath $tickerLog -Tail 60 | ForEach-Object { Write-Host $_ }
    }
    $global:LASTEXITCODE = 0
}

function Stop-Smoke {
    $ErrorActionPreference = "Continue"
    if ($env:HP_EXE) { & $env:HP_EXE ticker stop 2>&1 | Write-Host }
    & herdr server stop 2>&1 | Write-Host
    foreach ($id in @($env:HERDR_SERVER_PID, $env:TICKER_PID)) {
        if ($id -and (Get-Process -Id $id -ErrorAction SilentlyContinue)) {
            if ($IsWindows) {
                # Windows: the whole tree.
                & taskkill.exe /PID $id /T /F 2>&1 | Write-Host
            } else {
                Stop-Process -Id $id -Force -ErrorAction SilentlyContinue
            }
        }
    }
    $global:LASTEXITCODE = 0
}
