#!/bin/sh
# Makes the ElevenLabs browser client the call page loads (crates/web/src/webui/page/vendor/): one self-contained ES
# module of @elevenlabs/client with LiveKit inside it, and its audio worklet, so the page loads both from the board's
# own address and the web UI's script rule (`script-src 'self'`) stays as it is. Pinned versions; run it again only to
# move to newer ones, then commit what it wrote. Needs Node and npm; nothing it installs stays in the repo.
set -e
CLIENT=1.28.0; LIVEKIT=2.23.0; TYPES=0.25.0; ESBUILD=0.25.10
here=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
out="$here/crates/web/src/webui/page/vendor"
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
cd "$tmp"
npm init -y >/dev/null
npm i --silent --no-audit --no-fund "@elevenlabs/client@$CLIENT" "livekit-client@$LIVEKIT" "@elevenlabs/types@$TYPES" "esbuild@$ESBUILD"
echo 'export { Conversation } from "@elevenlabs/client";' > entry.js
npx esbuild entry.js --bundle --format=esm --platform=browser --target=es2020 --minify --legal-comments=none --outfile=bundle.js --log-level=warning
mkdir -p "$out"
{ echo "// The ElevenLabs client $CLIENT for the browser (LiveKit $LIVEKIT inside), one ES module the call page imports."; echo "// Do not edit: run the script scripts/voice-sdk.sh."; cat bundle.js; } > "$out/elevenlabs.js"
w=$(find node_modules/@elevenlabs/client -name 'rawAudioProcessor*.js' -path '*worklet*' | head -1)
[ -n "$w" ] || { echo "The client's audio worklet was not found." >&2; exit 1; }
{ echo "// The ElevenLabs client's audio worklet ($CLIENT), loaded from the board's own address instead of a blob."; echo "// Do not edit: run the script scripts/voice-sdk.sh."; cat "$w"; } > "$out/rawAudioProcessor.js"
wc -c "$out/elevenlabs.js" "$out/rawAudioProcessor.js"
