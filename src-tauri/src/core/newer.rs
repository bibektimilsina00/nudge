//! Is there a newer build than this one?
//!
//! The reason this ships in the *first* public build rather than the second:
//! there is no way to update somebody into having an updater. Whoever downloads
//! a build without one is pinned to it for good, and the only route off is
//! noticing a website, downloading again, and dragging it over the old one --
//! which is a thing people do once and then stop doing.
//!
//! What lives here is only the question *is this version newer*, because that is
//! the part worth testing and the part that is wrong in interesting ways. Asking
//! the network, downloading and swapping the bundle are the updater plugin's
//! job, and re-implementing them would be a second, worse copy of something that
//! already handles the signature check.

/// Compare two dotted versions, loosely.
///
/// Loosely because the two sides come from different places -- one compiled into
/// the binary, one typed into `publish.py` -- and they will not always have the
/// same number of parts. `0.2` and `0.2.0` are the same version, and a release
/// that is refused because somebody left off a zero is a bug nobody will look
/// for in the right place.
///
/// Anything non-numeric sorts as older than anything numeric, so `0.2.0-beta1`
/// does not read as newer than `0.2.0`. That is deliberately cruder than semver
/// and errs the safe way: the cost is a pre-release that does not offer itself,
/// and the cost of the other mistake is pushing a beta at everybody.
pub fn newer(have: &str, offered: &str) -> bool {
    let parts = |v: &str| -> Vec<i64> {
        v.trim()
            .trim_start_matches('v')
            .split(['.', '-', '+'])
            .map(|p| p.parse::<i64>().unwrap_or(-1))
            .collect()
    };
    let (a, b) = (parts(have), parts(offered));
    for i in 0..a.len().max(b.len()) {
        // A missing part is zero, so 0.2 and 0.2.0 compare equal.
        let (x, y) = (
            a.get(i).copied().unwrap_or(0),
            b.get(i).copied().unwrap_or(0),
        );
        if x != y {
            return y > x;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_higher_version_is_newer() {
        assert!(newer("0.1.0", "0.2.0"));
        assert!(newer("0.1.0", "0.1.1"));
        assert!(newer("0.9.9", "1.0.0"));
        // Ten is not less than nine, which string comparison would say it was.
        assert!(newer("0.9.0", "0.10.0"));
    }

    #[test]
    fn the_same_version_is_not_newer() {
        assert!(!newer("0.1.0", "0.1.0"));
        // The one an absent zero would break.
        assert!(!newer("0.2", "0.2.0"));
        assert!(!newer("0.2.0", "0.2"));
        assert!(!newer("0.1.0", "v0.1.0"));
    }

    #[test]
    fn an_older_version_is_never_offered() {
        assert!(!newer("0.2.0", "0.1.0"));
        assert!(!newer("1.0.0", "0.9.9"));
    }

    /// Errs towards not offering, which is the survivable mistake.
    #[test]
    fn a_pre_release_does_not_push_itself_at_everybody() {
        assert!(!newer("0.2.0", "0.2.0-beta1"));
        assert!(newer("0.2.0-beta1", "0.2.0"));
    }

    /// The server is not ours to trust blindly; nonsense must not read as new.
    #[test]
    fn nonsense_is_not_newer_than_a_real_version() {
        assert!(!newer("0.1.0", ""));
        assert!(!newer("0.1.0", "banana"));
        assert!(!newer("0.1.0", "0.0.1"));
    }
}
