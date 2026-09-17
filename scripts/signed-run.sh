#!/bin/sh
# Sign a binary with the project's pinned certificate, then run it.
#
# Cargo hands every `cargo run`/`cargo test` binary here first. Without this they
# ship the ad-hoc signature the linker puts on arm64 builds, whose only identity is
# a cdhash -- so the Keychain records "Always Allow" against one specific build and
# the next `cargo build` invalidates it. Six dead entries had piled up on
# nudge-github before anyone worked out why the password box kept coming back.
#
# Signing under the same identifier as the bundle is what avoids that, and it is
# deliberate: these binaries then satisfy the requirement the .app is already
# trusted by, so they need no grant of their own. The scope is the same either way
# -- anything built from this checkout can already read these tokens.
set -e
bin=$1
shift

id=$(cat "$(dirname "$0")/../.signing-identity" 2>/dev/null || true)
if [ -n "$id" ]; then
    # A missing certificate must not stop the binary running -- CI has no keychain.
    codesign --force --sign "$id" --identifier dev.nudge.app "$bin" 2>/dev/null \
      || echo "signed-run: could not sign as '$id'; expect a Keychain prompt" >&2
fi

exec "$bin" "$@"
