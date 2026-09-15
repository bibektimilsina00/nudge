# Releasing

Nudge as built today **runs on the machine that built it and nowhere else**. Ask
Gatekeeper what anybody else's Mac would say:

```
$ make ship-check
gatekeeper:
  src-tauri/target/release/bundle/macos/Nudge.app: rejected
  origin=Apple Development: Bibek Timilsina (6KVV6M8BNW)
```

`rejected` is the whole problem. An **Apple Development** certificate is for
running your own builds on your own machines; for anybody else, macOS refuses to
open it at all. Every feature in this repository is theoretical until this says
`accepted`.

## What is needed, and only you can get it

Three things, none of which live in this repository:

**1. A Developer ID Application certificate.** Requires an Apple Developer
Program membership (£79/$99 a year) on an *individual* or *organisation* account
you control. This machine currently has three Apple Development certificates and
one Apple Distribution certificate for TaggedWeb Inc — **none of them work**.
Apple Distribution is for the App Store, which is a different route with
different rules; Developer ID is the one for handing somebody a download.

Create it at developer.apple.com → Certificates → **Developer ID Application**.

**2. An app-specific password.** appleid.apple.com → Sign-In and Security →
App-Specific Passwords. Not your account password — notarisation will refuse it,
and the error does not say why.

**3. Your Team ID.** The ten characters in brackets at the end of the
certificate name, and on developer.apple.com → Membership.

## Then, locally

```sh
printf '%s' 'Developer ID Application: Your Name (ABCDE12345)' > .signing-identity-release
export APPLE_ID='you@example.com'
export APPLE_PASSWORD='xxxx-xxxx-xxxx-xxxx'
export APPLE_TEAM_ID='ABCDE12345'

make release
```

That builds, signs, uploads to Apple, waits for them to scan it, staples the
result, and then asks Gatekeeper the same question as above. It should answer
`accepted`.

`.signing-identity-release` is gitignored.

## Or in CI

`.github/workflows/release.yml` does the same thing on a tag, so a release is not
a person being available. It needs five repository secrets:

| Secret | What |
|---|---|
| `APPLE_CERTIFICATE` | the `.p12`, base64-encoded |
| `APPLE_CERTIFICATE_PASSWORD` | the password you set when exporting it |
| `APPLE_SIGNING_IDENTITY` | `Developer ID Application: Your Name (ABCDE12345)` |
| `APPLE_ID` | your Apple ID |
| `APPLE_PASSWORD` | the app-specific password |
| `APPLE_TEAM_ID` | `ABCDE12345` |

Export the certificate from Keychain Access (right-click → Export, as `.p12`),
then:

```sh
base64 -i Certificates.p12 | pbcopy
```

Run the workflow by hand once, from the Actions tab, **before** cutting a tag. A
signing pipeline that has only ever run once has never really run.

## Three things worth knowing

**Stapling is not optional.** Notarising gets you a ticket; stapling attaches it
to the bundle. Skip it and the app opens fine on the machine that notarised it —
because the ticket is cached there — and fails for everybody else until they
happen to be online. `scripts/release.sh` staples the `.app` and the `.dmg` and
then validates both.

**This builds for Apple silicon only.** `macos-14` is an arm64 runner, and there
is no universal binary here. An Intel Mac cannot open the result. Adding it means
a second runner and a `lipo` step, or `--target universal-apple-darwin` with both
Rust targets installed.

**Changing the signature voids permission grants.** macOS identifies an app by
its code signature, so the first Developer ID build is a different app as far as
TCC is concerned. Your own Screen Recording and Accessibility grants will need
giving again — that is expected, not a bug. `make reset-perms` clears them
deliberately.
