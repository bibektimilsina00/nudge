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

**1. A Developer ID Application certificate.** None of the four certificates on
this machine will do. Read out of the certificates themselves, the teams
available are:

| Team | Team ID | What is there |
|---|---|---|
| Bibek Timilsina | `9VA7RHWT84` | Apple Development only |
| Nuddg Inc | `7MATTTWP83` | Apple Development only |
| TaggedWeb Inc. | `3AKM83DNCV` | Apple Development **and** Apple Distribution |

Apple Development is for running your own builds on your own machines. Apple
Distribution is for the App Store, which is a different route with different
rules. Neither can be notarised for a download; only **Developer ID
Application** can.

An Apple Distribution certificate can only exist under a paid membership, so
TaggedWeb Inc has one — but shipping under it puts *"TaggedWeb Inc."* in the
Gatekeeper dialog somebody sees on first open. **Nuddg Inc** is the team this
belongs to; whether it needs paying for is the one thing to go and check.

Create it at developer.apple.com → Certificates, Identifiers & Profiles →
Certificates → **+** → **Developer ID Application**.

On an organisation account that row is greyed out with *"This operation can only
be performed by the Account Holder"* for everybody else, including Admins. Worth
reading carefully though: the row being **there at all** means the membership is
paid, because a free account does not show Developer ID options. A greyed-out
row is a role problem, not a billing one.

Three ways past it, cheapest first:

1. **Switch team.** An *individual* account has no separate Account Holder --
   you are it -- so if a personal membership is paid, the button is already live
   there.
2. **Have the Account Holder create it and send the `.p12`.** The role is not
   needed permanently; one file is. Keychain Access → right-click the
   certificate → Export → `.p12` with a password. This is what the CI workflow
   consumes anyway, so it is the ordinary setup rather than a workaround.
3. **Become the Account Holder**, which is a transfer the current one performs
   and which moves the legal agreements with it. Only worth it if Nudge is going
   to live under that team for good.

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
