//! The public key we ship must verify signatures made by the key we sign with.
//!
//! This is the failure that strands everybody. The private key lives in a
//! GitHub secret and the public key is compiled into `tauri.conf.json`, so
//! there is nothing in the ordinary build that notices when one is rotated and
//! the other is not. Everything keeps working -- the build succeeds, the
//! signature is produced, the endpoint serves it -- until a running copy tries
//! to install, fails verification, and the only route off that version is
//! downloading the app again by hand.
//!
//! `pairing.txt.sig` was made by the real signing key over `pairing.txt`. The
//! private key is not here and does not need to be: a signature is enough to
//! prove which key made it.
//!
//! **If this fails after rotating the updater key**, re-sign the fixture with
//! the new private key and commit both:
//!
//! ```sh
//! npx @tauri-apps/cli@2 signer sign -f .signing/updater.key -p "" \
//!     src-tauri/tests/fixtures/pairing.txt
//! ```
use minisign_verify::{PublicKey, Signature};

#[test]
fn the_shipped_public_key_matches_the_signing_key() {
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../tauri.conf.json")).expect("tauri.conf.json");
    let shipped = config["plugins"]["updater"]["pubkey"]
        .as_str()
        .expect("no updater pubkey in tauri.conf.json -- updates would never verify");

    // Tauri base64-wraps the minisign text format, for both the key and the
    // signature -- so each is unwrapped once before minisign sees it.
    let key = PublicKey::decode(&unwrap(shipped)).expect("the pubkey is not a minisign key");
    let signature = Signature::decode(&unwrap(include_str!("fixtures/pairing.txt.sig")))
        .expect("the fixture signature does not parse");

    key.verify(include_bytes!("fixtures/pairing.txt"), &signature, false)
        .expect(
            "the shipped public key does not verify a signature from the signing key -- \
             every update would fail at install and nobody could get off that version",
        );
}

/// Undo Tauri's base64 wrapper around minisign's own text format.
fn unwrap(wrapped: &str) -> String {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(wrapped.trim())
        .expect("not base64");
    String::from_utf8(bytes).expect("not text")
}
