# The plugin's build step on Windows (herdr-plugin.toml): puts the program at bin\run.exe, which is what herdr starts
# for every plugin command there (the commands say bin/run, and Windows finds bin\run.exe for it). The first of:
#   1. bin\run.exe already there, of this checkout's version (Cargo.toml) or newer (its version is in bin\run.exe.version);
#   2. the release file herdr-projects-windows-x86_64.exe of this version, downloaded from GitHub (gh for a private
#      repo), its checksum checked;
#   3. built from source with cargo, when Rust is installed.
# Then bin/run, the Linux and macOS launcher, is removed from this checkout: Windows would find that file before
# bin\run.exe. HERDR_PROJECTS_RELEASES names another place for the releases (a fork, a mirror, a folder as file:///…).
$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$root = Split-Path -Parent $here
function Say($m) { [Console]::Error.WriteLine("herdr-projects: $m") }
function Done {
  Remove-Item -Force -ErrorAction SilentlyContinue (Join-Path $here 'run')
  exit 0
}
function AtLeast($a, $b) {
  try { return [version]($a -replace '[^0-9.].*$', '') -ge [version]($b -replace '[^0-9.].*$', '') } catch { return $false }
}

$want = (Select-String -Path (Join-Path $root 'Cargo.toml') -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
$prog = Join-Path $here 'run.exe'
$have = Get-Content -ErrorAction SilentlyContinue "$prog.version" | Select-Object -First 1
if ((Test-Path $prog) -and $have -and (AtLeast $have $want)) { Done }

# Windows on Arm runs the x86_64 program too.
$name = 'herdr-projects-windows-x86_64.exe'
$base = if ($env:HERDR_PROJECTS_RELEASES) { $env:HERDR_PROJECTS_RELEASES.TrimEnd('/') } else { 'https://github.com/eliasstravik/herdr-projects/releases' }
$url = "$base/download/v$want/$name"
$tmp = Join-Path $here ".$name.$PID.download"
function Fetch($from, $to) {
  try {
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
    Invoke-WebRequest -UseBasicParsing -Uri $from -OutFile $to
    return $true
  } catch {}
  # A private repo's release is not handed out without a login: gh, logged in, fetches it.
  $gh = 'https://github.com/eliasstravik/herdr-projects/releases/download/'
  if ($from.StartsWith($gh) -and (Get-Command gh -ErrorAction SilentlyContinue)) {
    $rest = $from.Substring($gh.Length); $tag = $rest.Split('/')[0]; $file = $rest.Substring($tag.Length + 1)
    # What gh says on stderr is not a reason to stop this script (Windows PowerShell turns it into an error).
    $ErrorActionPreference = 'Continue'
    try { & gh release download $tag -R eliasstravik/herdr-projects -p $file -O $to --clobber 2>$null | Out-Null; return ($LASTEXITCODE -eq 0) } catch { return $false }
  }
  return $false
}

Say "downloading herdr-projects $want for Windows (once)"
if ((Fetch $url $tmp) -and (Fetch "$url.sha256" "$tmp.sha256")) {
  $sum = ((Get-Content "$tmp.sha256" | Select-Object -First 1) -split '\s+')[0].ToLower()
  $got = (Get-FileHash -Algorithm SHA256 $tmp).Hash.ToLower()
  Remove-Item -Force -ErrorAction SilentlyContinue "$tmp.sha256"
  if ($sum -eq $got) {
    # A running program's file cannot be replaced on Windows, but it can be renamed out of the way.
    if (Test-Path $prog) { Remove-Item -Force -ErrorAction SilentlyContinue "$prog.old"; Move-Item -Force $prog "$prog.old" }
    Move-Item -Force $tmp $prog
    Set-Content -NoNewline -Path "$prog.version" -Value "$want`n"
    Done
  }
  Say "the download does not match its checksum, so it is not used"
} else {
  Say "could not download $url (no network, no release v$want with a Windows file yet, or a private repo: log in with gh auth login). HERDR_PROJECTS_RELEASES names another place for the releases."
}
Remove-Item -Force -ErrorAction SilentlyContinue $tmp, "$tmp.sha256"

if (Get-Command cargo -ErrorAction SilentlyContinue) {
  Say 'building it from source instead: cargo build --release (a few minutes)'
  Push-Location $root
  $ErrorActionPreference = 'Continue'
  try { & cargo build --release --locked } catch { Say "cargo could not build it: $_" } finally { Pop-Location }
  $ErrorActionPreference = 'Stop'
  if ($LASTEXITCODE -eq 0) {
    if (Test-Path $prog) { Remove-Item -Force -ErrorAction SilentlyContinue "$prog.old"; Move-Item -Force $prog "$prog.old" }
    Copy-Item -Force (Join-Path $root 'target\release\herdr-projects.exe') $prog
    Set-Content -NoNewline -Path "$prog.version" -Value "$want`n"
    Done
  }
}
if (Test-Path $prog) { Say "using the herdr-projects $have already here, not $want"; Done }
Say "no program to run: try again later, or build it (Rust: https://rustup.rs) and run this again"
exit 1
