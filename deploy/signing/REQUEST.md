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

# Second thing, same visit: an App Store Connect API key

Signing the app is half of it. The other half is **notarisation** — Apple scans
the build and confirms it is not malware, and macOS checks that before letting
anyone open it. That needs credentials for the team, and an API key is the
cleanest kind: no password is shared, it works unattended, and it can be revoked
on its own without touching anything else.

## Steps

1. Go to **https://appstoreconnect.apple.com/access/integrations/api**

2. Select the **Team Keys** tab (not Individual Keys).

3. Click **+**, name it `Nudge Notarisation`, and set **Access** to
   **Developer**. That is the lowest role that can notarise — it cannot publish
   apps, see sales, or change team membership.

4. Click **Generate**, then **Download the API key**. You get a file named
   `AuthKey_XXXXXXXXXX.p8`.

   **Apple only lets this be downloaded once.** If the page is closed before
   downloading, the key has to be revoked and a new one made.

5. Send back three things:
   - the `AuthKey_XXXXXXXXXX.p8` file
   - the **Key ID** (10 characters, shown in the list)
   - the **Issuer ID** (a long uuid, shown at the top of the page)

## Is this one secret?

Yes — unlike the certificate, this `.p8` is a private key and should be sent
somewhere private rather than over ordinary email or chat. It can be revoked
from the same page at any time, instantly, with no effect on the certificate or
on builds already released.
