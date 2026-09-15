#!/usr/bin/env bash
#
# Hand the app to somebody without paying Apple first.
#
# Gatekeeper does not check every app -- it checks *quarantined* ones. macOS
# attaches that flag to anything arriving from a browser, a chat app or AirDrop,
# and it is the flag, not the signature, that produces "Nudge cannot be opened
# because Apple cannot check it for malicious software". Remove the flag and the
# app opens normally, signature intact.
#
# This is fine between people who know each other and wrong as a way to ship. The
# friend is taking your word for it, which is exactly what notarisation exists to
# replace -- see RELEASING.md.
set -euo pipefail
cd "$(dirname "$0")/.."

APP="src-tauri/target/release/bundle/macos/Nudge.app"
OUT="dist"

say() { printf '\n\033[1m%s\033[0m\n' "$*"; }

say "Building"
( cd src-tauri && cargo tauri build )
[ -d "${APP}" ] || { echo "No app at ${APP}" >&2; exit 1; }

rm -rf "${OUT}"; mkdir -p "${OUT}"

# `ditto`, not `zip`. A plain zip loses the symlinks and extended attributes a
# bundle is made of, and the signature does not survive the round trip -- which
# turns "unsigned developer" into "damaged and should be moved to the Bin", a
# much more alarming dialog for no reason.
say "Packing"
ditto -c -k --keepParent "${APP}" "${OUT}/Nudge.zip"

cat > "${OUT}/READ ME FIRST.txt" <<'TXT'
Nudge
=====

macOS will refuse to open this the first time, saying it cannot check it for
malicious software. That is not a fault in the app -- it means it has not been
through Apple's notarisation service, which costs money and is being sorted out.

To open it:

  1. Drag Nudge.app to your Applications folder.

  2. Open Terminal and paste this, then press return:

       xattr -dr com.apple.quarantine /Applications/Nudge.app

     It removes the "downloaded from the internet" flag. Nothing else.

  3. Open Nudge normally. It lives in your menu bar, not the Dock.

Then it will ask for two permissions, and it genuinely cannot work without
them:

  Screen Recording  -- so it can see what you are pointing at
  Accessibility     -- so it can click and type for you

Nudge's own Settings has a Permissions page with a button straight to each one.

Hold the Control key and talk to it.
TXT

say "Ready to send"
ls -lh "${OUT}" | sed 's/^/  /'
echo
echo "  Send both files. The instructions are the important half --"
echo "  without them it looks broken rather than unsigned."
