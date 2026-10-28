#!/bin/sh
# Shepherdr runs on Bun. herdr runs plugin commands with its own environment, so look in the usual places too. With
# none, get one pinned Bun (its checksum checked) into Shepherdr's own folder, once: the user installs nothing. Bun's
# folder goes on the PATH too, for package.json's scripts (bun run build).
run() {
  PATH="$(dirname "$1"):$PATH"
  export PATH
  exec "$@"
}
for b in "$(command -v bun 2>/dev/null)" "$HOME/.bun/bin/bun" /usr/local/bin/bun /opt/homebrew/bin/bun; do
  if [ -n "$b" ] && [ -x "$b" ]; then run "$b" "$@"; fi
done

VERSION=1.4.2
dir="${SHEPHERDR_HOME:-$HOME/.shepherdr}/bun/$VERSION"
if [ -x "$dir/bun" ]; then run "$dir/bun" "$@"; fi

say() { echo "Shepherdr: $*" >&2; }
need() {
  say "$1. Install Bun by hand: curl -fsSL https://bun.sh/install | bash"
  exit 1
}

case "$(uname -s)" in Linux) os=linux ;; Darwin) os=darwin ;; *) need "no Bun for $(uname -s)" ;; esac
case "$(uname -m)" in x86_64 | amd64) arch=x64 ;; aarch64 | arm64) arch=aarch64 ;; *) need "no Bun for $(uname -m)" ;; esac
# a Mac on Apple silicon running this shell under Rosetta: the native one
[ "$os$arch" = darwinx64 ] && [ "$(sysctl -n sysctl.proc_translated 2>/dev/null)" = 1 ] && arch=aarch64
target="$os-$arch"
if [ "$os" = linux ] && ldd --version 2>&1 | grep -qi musl; then target="$target-musl"; fi
# an x64 processor without AVX2 runs the baseline build
if [ "$arch" = x64 ]; then
  if [ "$os" = linux ]; then grep -q avx2 /proc/cpuinfo 2>/dev/null || target="$target-baseline"
  else sysctl -n machdep.cpu.leaf7_features 2>/dev/null | grep -qi avx2 || target="$target-baseline"; fi
fi
# from the release's SHASUMS256.txt
case "$target" in
  linux-x64) sum=36368faef7527875d5ffa52e53cd48021741f2a83eb6208a8dd64068d422a913 ;;
  linux-x64-baseline) sum=c678040f14fe0440eb839d37cbd0ce4c051a32da72806ac97de6a6aab6bf728f ;;
  linux-x64-musl) sum=4835eca59d6da70f4674f5642f6e459dcadab773695b2ed9922d131057989742 ;;
  linux-x64-musl-baseline) sum=76e1db84e98f22f78de0a87e309bfbbf297732847f9720db36750646c85c8c18 ;;
  linux-aarch64) sum=54328bbc2d9c8e0c9f892c544d66c57a83b84139e34909e5ee81758f1ac8fda7 ;;
  linux-aarch64-musl) sum=71760b6c8ea30623b81a4907cb815d48e2ea266f2e73e751534a44a0607950df ;;
  darwin-x64) sum=80520d7e17526308c9185d261679ac6d27798d3803a0e9f7ff9121ab8affb012 ;;
  darwin-x64-baseline) sum=bad5bbd6cf14d0980d115f5954c9ff904df619d5e994d2da1ffccd3f316300b0 ;;
  darwin-aarch64) sum=90987a3a16d7db556d886ac3d551e7b6d3edf0a1cf43acaed622e8676be1d12f ;;
  *) need "no Bun for $target" ;;
esac

say "Bun was not found: getting Bun $VERSION ($target) into $dir"
mkdir -p "$dir" || need "can't make $dir"
tmp=$(mktemp -d "$dir/.get.XXXXXX") || need "can't write to $dir"
trap 'rm -rf "$tmp"' EXIT
url="https://github.com/oven-sh/bun/releases/download/bun-v$VERSION/bun-$target.zip"
if command -v curl >/dev/null 2>&1; then curl -fsSL --connect-timeout 15 --max-time 600 -o "$tmp/bun.zip" "$url" || need "couldn't download $url"
elif command -v wget >/dev/null 2>&1; then wget -q -T 30 -O "$tmp/bun.zip" "$url" || need "couldn't download $url"
else need "neither curl nor wget is installed"; fi
got=$({ sha256sum "$tmp/bun.zip" || shasum -a 256 "$tmp/bun.zip" || openssl dgst -sha256 -r "$tmp/bun.zip"; } 2>/dev/null | cut -d ' ' -f 1)
[ "$got" = "$sum" ] || need "the download of $url has the wrong checksum"
if command -v unzip >/dev/null 2>&1; then unzip -q -o "$tmp/bun.zip" -d "$tmp" || need "couldn't unpack $url"
elif command -v python3 >/dev/null 2>&1; then python3 -I -m zipfile -e "$tmp/bun.zip" "$tmp" || need "couldn't unpack $url"
else need "neither unzip nor python3 is installed to unpack Bun"; fi
chmod +x "$tmp/bun-$target/bun" && "$tmp/bun-$target/bun" --version >/dev/null 2>&1 || need "the Bun it got doesn't run here"
# one step, so a second start at the same time finds a whole Bun or none
mv -f "$tmp/bun-$target/bun" "$dir/bun" || need "couldn't put Bun in $dir"
rm -rf "$tmp"
trap - EXIT
run "$dir/bun" "$@"
