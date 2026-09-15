#!/usr/bin/env bash
#
# Build something another person can open.
#
# Nudge signed with an "Apple Development" certificate runs on the machine that
# built it and nowhere else -- Gatekeeper rejects it outright, which `spctl` will
# say in one line. Everything the app does is theoretical until this script has
# run successfully, because until then there is nobody to do it for.
#
# Three steps, and all three are required:
#
#   sign       with a Developer ID Application certificate, hardened runtime on
#   notarise   upload to Apple, who scan it and return a ticket
#   staple     attach that ticket to the bundle, so it opens offline
#
# Skipping the last one is the classic mistake: it works on the machine that
# notarised it, because the ticket is cached, and fails for everybody else until
# they happen to be online.
set -euo pipefail

cd "$(dirname "$0")/.."

say() { printf '\n\033[1m%s\033[0m\n' "$*"; }
die() { printf '\n\033[31m%s\033[0m\n' "$*" >&2; exit 1; }

# --- what this needs, checked before a twenty-minute build ------------------
#
# Failing here costs seconds. Failing after the build costs the build, and
# notarytool's own error for the wrong certificate type is famously unhelpful.

: "${APPLE_SIGNING_IDENTITY:=$(cat .signing-identity-release 2>/dev/null || true)}"
[ -n "${APPLE_SIGNING_IDENTITY}" ] || die \
"No Developer ID certificate chosen.

  Put its full name in .signing-identity-release, or set
  APPLE_SIGNING_IDENTITY. It has to be a *Developer ID Application*
  certificate -- an Apple Development one cannot be notarised.

  What this machine has:
$(security find-identity -v -p codesigning | sed 's/^/  /')"

case "${APPLE_SIGNING_IDENTITY}" in
  "Developer ID Application:"*) ;;
  *) die "\"${APPLE_SIGNING_IDENTITY}\" is not a Developer ID Application certificate.

  Apple Development and Apple Distribution certificates cannot be
  notarised for direct download. Only Developer ID can." ;;
esac

for needed in APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID; do
  [ -n "${!needed:-}" ] || die \
"${needed} is not set.

  Notarising needs an Apple ID, an app-specific password for it
  (appleid.apple.com > Sign-In and Security > App-Specific Passwords --
  not the account password), and the Team ID from the certificate.

    export APPLE_ID='you@example.com'
    export APPLE_PASSWORD='xxxx-xxxx-xxxx-xxxx'
    export APPLE_TEAM_ID='ABCDE12345'"
done

export APPLE_SIGNING_IDENTITY

say "Signing as ${APPLE_SIGNING_IDENTITY}"

# --- build -----------------------------------------------------------------

say "Building"
( cd src-tauri && cargo tauri build )

APP="src-tauri/target/release/bundle/macos/Nudge.app"
DMG="$(ls -t src-tauri/target/release/bundle/dmg/*.dmg 2>/dev/null | head -1 || true)"
[ -d "${APP}" ] || die "No app at ${APP}"

# Checked rather than assumed: Tauri signs during the build, and a silent
# fallback to a different certificate is exactly the failure this script exists
# to catch. It has happened -- see the Makefile on why the identity is pinned.
say "Checking what actually got signed"
codesign -dv --verbose=2 "${APP}" 2>&1 | grep -E 'Authority|flags' | sed 's/^/  /'
codesign -dv "${APP}" 2>&1 | grep -q 'flags=.*runtime' \
  || die "Hardened runtime is off. Notarisation will refuse it."
codesign -dv --verbose=2 "${APP}" 2>&1 | grep -q 'Authority=Developer ID Application' \
  || die "Signed with the wrong certificate. Notarisation will refuse it."

# --- notarise --------------------------------------------------------------
#
# The zip is a transport format and nothing else. The ticket is stapled to the
# .app afterwards, not to the zip, so this file is thrown away.

say "Notarising -- Apple usually takes a few minutes"
ZIP="$(mktemp -d)/Nudge.zip"
ditto -c -k --keepParent "${APP}" "${ZIP}"

xcrun notarytool submit "${ZIP}" \
  --apple-id "${APPLE_ID}" \
  --password "${APPLE_PASSWORD}" \
  --team-id "${APPLE_TEAM_ID}" \
  --wait

# --- staple ----------------------------------------------------------------

say "Stapling"
xcrun stapler staple "${APP}"
[ -n "${DMG}" ] && xcrun stapler staple "${DMG}"

# --- prove it ---------------------------------------------------------------
#
# The whole point. `spctl` answers the question another person's Mac will ask,
# and this is the line that turns "it should work" into "it does".

say "What another Mac will say"
spctl -a -vvv -t install "${APP}" 2>&1 | sed 's/^/  /'
xcrun stapler validate "${APP}" 2>&1 | sed 's/^/  /'

say "Ready"
echo "  ${APP}"
[ -n "${DMG}" ] && echo "  ${DMG}"
