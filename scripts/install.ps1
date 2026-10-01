# Puts the herdr-projects binary at target\release\herdr-projects.exe on Windows.
#
# The Windows twin of scripts/install.sh. Herdr runs this as the plugin's
# build step, and `herdr-projects update` runs it in a linked checkout. It
# downloads herdr-projects-x86_64-pc-windows-msvc.exe of the release named by
# herdr-plugin.toml's `version`, checks it against the release's SHA256SUMS,
# and falls back to `cargo build --release --locked` when there is no such
# binary or it cannot be verified. Then, during `herdr plugin install`, it
# writes the `herdr-projects.cmd` command shim (what scripts/link-command.sh
# does on macOS and Linux).
#
# A running .exe cannot be overwritten on Windows, but it can be renamed: the
# old binary is moved aside to herdr-projects.exe.old-<time> before the new
# one takes its place, and those leftovers are removed on the next install.
#
#   $env:HERDR_PROJECTS_BUILD = 'source'        always build from source
#   $env:HERDR_PROJECTS_DOWNLOAD_URL = '<url>'  download from <url>/v<version>/
#                                               instead of the GitHub release of `origin`
#
# Written for Windows PowerShell 5.1, which is what `powershell` runs.
Set-StrictMode -Version 2
$ErrorActionPreference = 'Continue'
$ProgressPreference = 'SilentlyContinue'

Set-Location -LiteralPath (Split-Path -Parent $PSScriptRoot)
$checkout = (Get-Location).ProviderPath
$release = Join-Path $checkout 'target\release'
$exe = Join-Path $release 'herdr-projects.exe'
$script:tmp = $null

function Say([string] $message) {
  [Console]::Error.WriteLine("herdr-projects install: $message")
}

function Remove-Download {
  if ($script:tmp -and (Test-Path -LiteralPath $script:tmp)) {
    Remove-Item -LiteralPath $script:tmp -Recurse -Force -ErrorAction SilentlyContinue
  }
}

# Removes binaries moved aside by earlier installs. One still running stays,
# and goes next time.
function Remove-OldBinaries {
  Get-ChildItem -LiteralPath $release -Filter 'herdr-projects.exe.old-*' -ErrorAction SilentlyContinue |
    ForEach-Object { Remove-Item -LiteralPath $_.FullName -Force -ErrorAction SilentlyContinue }
}

# Moves the current binary aside, since it may be running; returns where to,
# or $null when there was none.
function Move-Aside {
  if (-not (Test-Path -LiteralPath $exe)) { return $null }
  $aside = "$exe.old-$([DateTime]::UtcNow.ToString('yyyyMMddHHmmssfff'))"
  try {
    Move-Item -LiteralPath $exe -Destination $aside -Force -ErrorAction Stop
  } catch {
    Say "could not move the old $exe aside: $($_.Exception.Message)"
    exit 1
  }
  return $aside
}

# Writes <bin>\herdr-projects.cmd during `herdr plugin install`, the same rules
# as scripts/link-command.sh and the binary's own `command_link`. Herdr builds
# in <plugins>\.tmp-install-*\checkout and, once that passes, moves the
# checkout to <plugins>\github\<id>-<first 12 hex digits of sha256(id)>, so the
# shim names the binary's final place. Anywhere else it does nothing: the
# binary itself relinks at every plugin start and on `doctor --fix`. It never
# replaces a file that is not our shim, or a shim that points outside Herdr's
# plugin folder and still works.
function Write-CommandShim {
  if ($checkout -notmatch '^(.*)[\\/]\.tmp-install-[^\\/]*[\\/]checkout$') { return }
  $plugins = $Matches[1]
  $sha = [System.Security.Cryptography.SHA256]::Create()
  $digest = $sha.ComputeHash([System.Text.Encoding]::UTF8.GetBytes('herdr-projects'))
  $hash = (-join ($digest | ForEach-Object { $_.ToString('x2') })).Substring(0, 12)
  $target = Join-Path $plugins "github\herdr-projects-$hash\target\release\herdr-projects.exe"

  if ($env:XDG_BIN_HOME) {
    $bin = $env:XDG_BIN_HOME
  } else {
    $home_dir = if ($env:HOME) { $env:HOME } else { $env:USERPROFILE }
    if (-not $home_dir) { Say 'neither HOME nor USERPROFILE is set, so no command shim was written'; return }
    $bin = Join-Path $home_dir '.local\bin'
  }
  $shim = Join-Path $bin 'herdr-projects.cmd'

  if (Test-Path -LiteralPath $shim) {
    $text = [System.IO.File]::ReadAllText($shim).TrimEnd("`r", "`n")
    if ($text -notmatch '^@"([^"\r\n]*)" %\*$') {
      Say "left $shim alone: it is not this plugin's command shim"
      return
    }
    $current = $Matches[1]
    if (-not $current.StartsWith($plugins + '\', [StringComparison]::OrdinalIgnoreCase) -and (Test-Path -LiteralPath $current)) {
      Say "left $shim alone: it runs $current"
      return
    }
  }
  try {
    New-Item -ItemType Directory -Force -Path $bin -ErrorAction Stop | Out-Null
    # Write under a temporary name, then rename over: never a moment without a command.
    $partial = Join-Path $bin ".herdr-projects.cmd.$PID"
    [System.IO.File]::WriteAllText($partial, "@`"$target`" %*`r`n", (New-Object System.Text.UTF8Encoding $false))
    Move-Item -LiteralPath $partial -Destination $shim -Force -ErrorAction Stop
  } catch {
    Remove-Item -LiteralPath $partial -Force -ErrorAction SilentlyContinue
    Say "could not write $shim"
    return
  }
  $user_path = [Environment]::GetEnvironmentVariable('Path', 'User')
  $on_path = @("$env:Path;$user_path" -split ';' | Where-Object { $_ -and ($_.TrimEnd('\') -eq $bin.TrimEnd('\')) }).Count -gt 0
  if ($on_path) {
    Say "linked $shim"
  } else {
    Say "linked $shim, but $bin is not on your PATH: add it in System Properties > Environment Variables"
  }
}

function Build-FromSource([string] $reason) {
  Remove-Download
  Say $reason
  Say 'building from source instead: cargo build --release --locked (this takes a minute or two)'
  if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Say 'cargo is not installed. Install Rust 1.89 or newer (https://rustup.rs) with the Visual Studio C++ build tools, then install again.'
    exit 1
  }
  New-Item -ItemType Directory -Force -Path $release | Out-Null
  Remove-OldBinaries
  # Cargo writes target\release\herdr-projects.exe in place, which fails while it runs.
  $aside = Move-Aside
  & cargo build --release --locked
  $code = $LASTEXITCODE
  if ($code -ne 0) {
    if ($aside -and -not (Test-Path -LiteralPath $exe)) {
      Move-Item -LiteralPath $aside -Destination $exe -Force -ErrorAction SilentlyContinue
    }
    exit $code
  }
  Write-CommandShim
  exit 0
}

if ($env:HERDR_PROJECTS_BUILD -eq 'source') {
  Build-FromSource 'HERDR_PROJECTS_BUILD=source is set'
}

$version = $null
foreach ($line in (Get-Content -LiteralPath 'herdr-plugin.toml' -ErrorAction SilentlyContinue)) {
  if ($line -match '^version *= *"([^"]*)"') { $version = $Matches[1]; break }
}
if (-not $version) { Build-FromSource 'herdr-plugin.toml has no version' }
$tag = "v$version"

# Windows on Arm runs the x86_64 binary, as it runs Herdr itself.
$arch = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
if ($arch -notin @('AMD64', 'ARM64')) { Build-FromSource "there is no prebuilt binary for $arch" }
$asset = 'herdr-projects-x86_64-pc-windows-msvc.exe'

# A prebuilt binary matches only the release commit itself. A checkout with
# local changes, or one on a commit after the release, builds what it has.
$origin = $null
if (Test-Path -LiteralPath '.git') {
  if (-not (Get-Command git -ErrorAction SilentlyContinue)) { Build-FromSource 'git is not installed to check this checkout' }
  $status = & git status --porcelain --untracked-files=no 2>$null
  if ($status) { Build-FromSource 'this checkout has uncommitted changes' }
  $head = & git rev-parse HEAD 2>$null
  $release_commit = & git rev-parse -q --verify "refs/tags/$tag^{commit}" 2>$null
  if (-not $release_commit) {
    # `^{}` is the commit an annotated tag points at; a lightweight tag has none.
    $env:GIT_TERMINAL_PROMPT = '0'
    $sha = $null; $peeled = $null
    foreach ($ref in @(& git ls-remote origin "refs/tags/$tag" "refs/tags/$tag^{}" 2>$null)) {
      $fields = $ref -split "`t"
      if ($fields.Count -lt 2) { continue }
      if ($fields[1].EndsWith('^{}')) { $peeled = $fields[0] } else { $sha = $fields[0] }
    }
    $release_commit = if ($peeled) { $peeled } else { $sha }
  }
  if (-not $release_commit) {
    Build-FromSource "could not find the $tag tag here or on origin"
  } elseif ($head -ne $release_commit) {
    Build-FromSource "this checkout is not the $tag release commit"
  }
  $origin = & git remote get-url origin 2>$null
}

if ($env:HERDR_PROJECTS_DOWNLOAD_URL) {
  $base = "$($env:HERDR_PROJECTS_DOWNLOAD_URL.TrimEnd('/'))/$tag"
} else {
  $repo = $null
  if ($origin -and ($origin -match 'github\.com[:/]([^/]+/[^/]+)$')) { $repo = $Matches[1] -replace '\.git$', '' }
  if (-not $repo) { $repo = 'eliasstravik/herdr-projects' }
  $base = "https://github.com/$repo/releases/download/$tag"
}

# Windows PowerShell 5.1 does not offer TLS 1.2 by default.
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
function Get-Download([string] $url, [string] $out) {
  for ($try = 0; $try -lt 3; $try++) {
    try {
      Invoke-WebRequest -Uri $url -OutFile $out -UseBasicParsing -TimeoutSec 60 -ErrorAction Stop
      return $true
    } catch {
      Start-Sleep -Seconds 1
    }
  }
  return $false
}

New-Item -ItemType Directory -Force -Path $release | Out-Null
$script:tmp = Join-Path $release ".download.$PID"
Remove-Download
New-Item -ItemType Directory -Force -Path $script:tmp | Out-Null

Say "downloading $asset $tag"
$sums = Join-Path $script:tmp 'SHA256SUMS'
if (-not (Get-Download "$base/SHA256SUMS" $sums)) { Build-FromSource "could not download $base/SHA256SUMS" }
$expected = $null
foreach ($line in (Get-Content -LiteralPath $sums)) {
  $fields = $line -split '\s+', 2
  if ($fields.Count -eq 2 -and ($fields[1] -eq $asset -or $fields[1] -eq "*$asset")) { $expected = $fields[0].ToLowerInvariant() }
}
if (-not $expected) { Build-FromSource "the $tag release has no $asset" }
$download = Join-Path $script:tmp $asset
if (-not (Get-Download "$base/$asset" $download)) { Build-FromSource "could not download $base/$asset" }
$actual = (Get-FileHash -LiteralPath $download -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actual -ne $expected) {
  Build-FromSource "the downloaded $asset does not match its SHA256SUMS entry (got $actual, expected $expected)"
}

$reported = ''
# A binary that does not start throws here; that leaves $reported empty.
try { $reported = [string](& $download --version 2>$null | Out-String).Trim() } catch { }
if (-not ($reported -eq "herdr-projects $version" -or $reported.StartsWith("herdr-projects $version+", [StringComparison]::Ordinal))) {
  $said = if ($reported) { $reported } else { 'nothing' }
  Build-FromSource "the downloaded binary did not run or is not $version (it said: $said)"
}

Remove-OldBinaries
$aside = Move-Aside
try {
  Move-Item -LiteralPath $download -Destination $exe -Force -ErrorAction Stop
} catch {
  if ($aside) { Move-Item -LiteralPath $aside -Destination $exe -Force -ErrorAction SilentlyContinue }
  Build-FromSource 'could not move the binary into target\release'
}
Remove-Download
Say "installed the prebuilt $asset $tag"
Write-CommandShim
exit 0
