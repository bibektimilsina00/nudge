<!-- Sent to the Account Holder of the Apple team Nudge ships under. Kept here
     because the certificate expires after five years and this will be needed
     again, by somebody who will not remember any of it.

     The CSR this refers to lives in `.signing/`, which is gitignored, beside
     the private key it pairs with. Do not generate a second one: a cert issued
     against one CSR cannot be used with another CSR's key, and the failure
     reads as an unrelated codesign error. Check they match first:

       openssl rsa -in .signing/nudge-developer-id.key -pubout | openssl sha256
       openssl req -in .signing/nudge-developer-id.certSigningRequest -noout -pubkey | openssl sha256
-->

# Creating the Developer ID certificate for Nudge

This takes about two minutes and needs no security decisions from you. You will
upload one file and download another. **No password is involved and nothing
secret is being sent to you or by you** — the private key stays on Bibek's Mac
and never leaves it. The file you download is a public certificate.

## What you need

The attached file: `nudge-developer-id.certSigningRequest`

## Steps

1. Sign in at **https://developer.apple.com/account/resources/certificates/list**
   as the Account Holder for **Nuddg Inc**.

2. Click the **+** button next to "Certificates".

3. Under **Software**, choose **Developer ID Application**, then **Continue**.

4. If it asks about a profile type, choose **G2 Sub-CA (Xcode 11 or later)**.
   Then **Continue**.

5. **Choose File** → select `nudge-developer-id.certSigningRequest` → **Continue**.

6. Click **Download**. You get a file called `developerID_application.cer`.

7. Send that `.cer` file back.

That is everything. You do not need to install it, open it, or keep it.

## What this permits

The certificate lets Nudge be signed and notarised so that macOS will open it on
other people's Macs. Without it, macOS refuses to launch the app at all —
anybody who downloads it sees "Nudge is damaged and can't be opened".

It does not give anyone access to your Apple account, and it can be revoked at
any time from the same page.

## If the button is greyed out

Then the account is not the Account Holder, or the membership has lapsed. The
row appearing at all normally means the membership is paid and it is only the
role that is missing.

---

<!-- An App Store Connect API key used to be asked for here as well, for the
     notarisation half. It turned out not to be needed: an app-specific password
     made by anybody on the team notarises fine, and that was checked rather than
     assumed --

       xcrun notarytool history --apple-id <you> --password <app-specific> \
         --team-id 7MATTTWP83

     App Manager was enough. If a future role cannot notarise, that command says
     so in one line (HTTP 401) and the API key is the fallback: App Store Connect
     → Integrations → Team Keys → role Developer. The .p8 downloads exactly once.
-->
