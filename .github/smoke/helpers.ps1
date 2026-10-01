# Generic helpers for smoke.yml, dot-sourced by checks.ps1. Runs under pwsh
# on Windows, macOS and Linux.
$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

# What differs by platform, in one place.
$Exe = if ($IsWindows) { ".exe" } else { "" }
# Herdr's config folder: %APPDATA%\herdr on Windows, ~/.config/herdr on Unix.
$ConfigHome = if ($IsWindows) { $env:APPDATA } else { Join-Path $HOME ".config" }

# Runs a native command, prints its output, and throws on a non-zero exit.
function Invoke-Checked {
    param([string] $Command, [string[]] $Arguments)
    Write-Host "> $Command $($Arguments -join ' ')"
    $saved = $ErrorActionPreference
    try {
        $ErrorActionPreference = "Continue"
        $output = & $Command @Arguments 2>&1 | ForEach-Object { "$_" }
        $code = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $saved
    }
    $output | ForEach-Object { Write-Host "  $_" }
    if ($code -ne 0) {
        throw "exit code $code`: $Command $($Arguments -join ' ')"
    }
    $global:LASTEXITCODE = 0
    return ($output -join "`n")
}

# Runs a native command and returns its output and exit code, never throwing.
function Invoke-Quiet {
    param([string] $Command, [string[]] $Arguments)
    $saved = $ErrorActionPreference
    try {
        $ErrorActionPreference = "Continue"
        $output = & $Command @Arguments 2>&1 | ForEach-Object { "$_" }
        $code = $LASTEXITCODE
    } catch {
        $output = @($_.Exception.Message)
        $code = 1
    } finally {
        $ErrorActionPreference = $saved
    }
    $global:LASTEXITCODE = 0
    return [pscustomobject]@{ Text = ($output -join "`n"); Code = $code }
}

# Polls $Condition (a script block) until it returns something truthy, and
# returns that; throws with $What after $Seconds.
function Wait-Until {
    param([string] $What, [int] $Seconds, [scriptblock] $Condition, [int] $IntervalMs = 1000)
    $deadline = (Get-Date).AddSeconds($Seconds)
    while ($true) {
        $result = $null
        try { $result = & $Condition } catch { Write-Host "  (poll: $($_.Exception.Message))" }
        if ($result) {
            return $result
        }
        if ((Get-Date) -ge $deadline) {
            throw "timed out after $Seconds s waiting for: $What"
        }
        Start-Sleep -Milliseconds $IntervalMs
    }
}

# One row per check in $env:SMOKE_RESULTS, a file in RUNNER_TEMP. Each step
# has its own GITHUB_STEP_SUMMARY and GitHub shows them as separate blocks,
# so a table split across steps renders as text: Write-Summary turns the rows
# into one table at the end of the job.
function Add-Result {
    param([string] $Check, [bool] $Passed, [string] $Detail = "")
    $mark = if ($Passed) { "✅ pass" } else { "❌ FAIL" }
    $cells = @($Check, $mark, $Detail) | ForEach-Object { ($_ -replace "\s*`r?`n\s*", " " -replace '\|', '\|').Trim() }
    Add-Content -LiteralPath $env:SMOKE_RESULTS -Value "| $($cells -join ' | ') |" -Encoding utf8
}

# The job summary: one table with every result, in one step, so it renders.
function Write-Summary {
    $rows = @()
    if ($env:SMOKE_RESULTS -and (Test-Path -LiteralPath $env:SMOKE_RESULTS)) {
        $rows = @(Get-Content -LiteralPath $env:SMOKE_RESULTS -Encoding utf8 | Where-Object { $_.Trim() })
    }
    $lines = @("### Smoke checks on $env:RUNNER_OS", "", "| Check | Result | Detail |", "|---|---|---|") + $rows
    # A step outside Invoke-Check (setup, the server) failed: say so in the table.
    if ($env:JOB_STATUS -and $env:JOB_STATUS -ne "success" -and -not ($rows -match "❌")) {
        $lines += "| (a step that is not a check) | ❌ FAIL | job $env:JOB_STATUS; see the failed step's log |"
    }
    Add-Content -LiteralPath $env:GITHUB_STEP_SUMMARY -Value ($lines -join "`n") -Encoding utf8
}

# Runs one check: the body throws to fail. Records the result in the job
# summary and rethrows, so the first failing check stops the job.
function Invoke-Check {
    param([string] $Check, [scriptblock] $Body)
    try {
        $detail = (& $Body | Select-Object -Last 1)
    } catch {
        $message = $_.Exception.Message
        Add-Result $Check $false $message
        Write-Host "::error title=$Check::$($message -replace "`r?`n", ' ')"
        throw
    }
    Add-Result $Check $true "$detail"
    Write-Host "PASS: $Check $detail"
}

# herdr-projects from this checkout's build. No param block, so options
# such as --goal reach the binary as they are.
function Hp {
    Invoke-Checked $env:HP_EXE ([string[]] $args)
}

function Get-StubLog {
    param([string] $PaneId)
    $file = Join-Path $env:STUB_LOG_DIR "$($PaneId -replace '[:\\/]', '-').log"
    if (Test-Path -LiteralPath $file) { return Get-Content -LiteralPath $file -Raw -Encoding utf8 }
    return ""
}

function Get-ThreadRecord {
    param([string] $Slug, [string] $Id)
    $out = Invoke-Quiet $env:HP_EXE @("thread", "show", $Slug, $Id, "--json")
    if ($out.Code -ne 0) { return $null }
    return $out.Text | ConvertFrom-Json
}

# Types a line into a herdr pane's shell (text and Enter).
function Send-PaneLine {
    param([string] $PaneId, [string] $Text)
    Invoke-Checked herdr @("pane", "run", $PaneId, $Text) | Out-Null
}

function Read-Pane {
    param([string] $PaneId)
    $out = Invoke-Quiet herdr @("pane", "read", $PaneId, "--source", "recent-unwrapped", "--lines", "60", "--format", "text")
    return $out.Text
}
