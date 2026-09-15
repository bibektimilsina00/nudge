//! Reading a web page as text, instead of looking at a picture of one.
//!
//! The most expensive thing Nudge does is use the screen. Answering "what's the
//! weather in Kathmandu" by opening a browser costs a launch, a 2.2 second wait
//! for the page to settle, a full-display screenshot, a JPEG encode, a ~130KB
//! upload and a vision model reading a number off a picture -- and it puts a
//! window on somebody's screen to do it.
//!
//! One HTTP request and a few KB of text answers the same question. This is the
//! third turn of the same screw: prefer the keyboard shortcut to the menu,
//! prefer the command to the click, and now **prefer the data to the picture of
//! the data**. Anything a page's text can answer should never touch the screen.
use crate::error::{Error, Result};

/// How long to wait for a page. Past this it is faster to give up and say so.
const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(12);
/// How much of the page reaches the model. A long article is still an article
/// after this; a page that is longer is usually a listing nobody meant to read.
const MAX_CHARS: usize = 12_000;
/// Refuse a download rather than pulling it into memory to throw away.
const MAX_BYTES: u64 = 5 * 1024 * 1024;

/// Only the public web, and only over http.
///
/// The same shape as `launch::open_url`, and here for a sharper reason: `open`
/// hands a URL to a browser that has its own idea of what is safe, while this
/// makes the request itself, from inside the app, with whatever the network can
/// reach. A machine on a home network can reach its own router.
fn refuse(url: &str) -> Option<String> {
    let lower = url.trim().to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return Some("only http and https".into());
    }
    if url.len() > 2048 || url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Some("that does not look like a URL".into());
    }
    // Authority is everything after "//" and before the path, query or fragment;
    // the host is what is left after dropping any user:pass@ and the port.
    let rest = &lower[lower.find("//")? + 2..];
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let after_userinfo = authority.rsplit('@').next().unwrap_or("");
    // An IPv6 literal is bracketed and full of colons, so the port cannot be
    // found by splitting on ":" -- `[::1]:8080` would come back as "[".
    let host = match after_userinfo.strip_prefix('[') {
        Some(v6) => v6.split(']').next().unwrap_or(""),
        None => after_userinfo.split(':').next().unwrap_or(""),
    };
    if host.is_empty() {
        return Some("no host in that URL".into());
    }
    // Nothing on this machine or this network. A model that has been told to
    // "check the printer" should not be able to reach one, and `localhost` is
    // where a development server with somebody's database sits.
    const PRIVATE: &[&str] = &[
        "localhost",
        "127.",
        "0.0.0.0",
        "10.",
        "192.168.",
        "169.254.",
        // IPv6 loopback and link-local, without their brackets by this point.
        "::1",
        "fe80:",
        "fc00:",
        "fd",
    ];
    if PRIVATE.iter().any(|p| host.starts_with(p))
        || host.ends_with(".local")
        || host.ends_with(".internal")
        // 172.16.0.0/12
        || (host.starts_with("172.")
            && host
                .split('.')
                .nth(1)
                .and_then(|o| o.parse::<u8>().ok())
                .is_some_and(|o| (16..=31).contains(&o)))
    {
        return Some(format!("{host} is on this machine or this network"));
    }
    None
}

/// Fetch a page and return its readable text.
pub async fn read(url: &str) -> Result<String> {
    if let Some(why) = refuse(url) {
        return Err(Error::Click(format!("won't fetch {url}: {why}")));
    }
    let client = reqwest::Client::builder()
        .timeout(TIMEOUT)
        // Some sites serve a very different page to something that looks like a
        // script. Identifying honestly as Nudge and accepting the consequences
        // is better than pretending to be Chrome.
        .user_agent("Nudge/0.1 (+https://github.com/nudge)")
        .build()
        .map_err(|e| Error::Click(e.to_string()))?;

    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| Error::Click(format!("couldn't reach {url}: {e}")))?;
    if !resp.status().is_success() {
        return Err(Error::Click(format!("{url} returned {}", resp.status())));
    }
    if resp.content_length().is_some_and(|n| n > MAX_BYTES) {
        return Err(Error::Click(format!("{url} is too big to read")));
    }
    let kind = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();
    let body = resp
        .text()
        .await
        .map_err(|e| Error::Click(format!("couldn't read {url}: {e}")))?;

    let text = if kind.contains("html") {
        readable(&body, url)
    } else {
        // JSON, plain text, CSV. Already the thing we wanted.
        body
    };
    Ok(trim(&text))
}

/// Methods that only ask. Allowed without anybody granting anything, because
/// they are what `read` already does under another name.
const READING: &[&str] = &["GET", "HEAD"];

/// Headers a caller may not set.
///
/// `Host` would let a request aimed at an allowed name arrive somewhere else
/// entirely, which walks straight around the check in `refuse`. The rest are
/// hop-by-hop headers that belong to the connection rather than to the request,
/// and setting them by hand breaks the client rather than achieving anything.
const NOT_YOURS: &[&str] = &[
    "host",
    "content-length",
    "connection",
    "transfer-encoding",
    "upgrade",
    "proxy-authorization",
];

/// Make a request with a method, headers and a body.
///
/// Separate from [`read`] rather than folded into it, because they want opposite
/// things from a reply. `read` wants a page as prose and treats anything that is
/// not a success as a failure. This wants the reply itself: an API answering 422
/// with a JSON explanation has *answered*, and turning that into an error throws
/// away the only useful part.
///
/// `granted` is [`crate::core::reach::Grant::Http`]. Without it this is still
/// useful -- an authenticated GET against a real API is most of what people want
/// -- and anything that could change something on the other end is refused.
pub async fn request(
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: Option<&str>,
    granted: bool,
) -> Result<String> {
    let verb = method.trim().to_ascii_uppercase();
    if let Some(why) = refuse_request(&verb, url, headers, granted) {
        return Err(Error::Click(format!("won\u{27}t {verb} {url}: {why}")));
    }

    let client = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .user_agent("Nudge/0.1 (+https://github.com/nudge)")
        // Followed by default, and a redirect can point anywhere -- including
        // back at this machine, which is the whole thing `refuse` is for. Each
        // hop is checked by the same rules as the first.
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            match refuse(attempt.url().as_str()) {
                Some(_) => attempt.stop(),
                None if attempt.previous().len() > 5 => attempt.stop(),
                None => attempt.follow(),
            }
        }))
        .build()
        .map_err(|e| Error::Click(e.to_string()))?;

    let m = reqwest::Method::from_bytes(verb.as_bytes())
        .map_err(|_| Error::Click(format!("{verb} is not an HTTP method")))?;
    let mut req = client.request(m, url);
    for (k, v) in headers {
        req = req.header(k, v);
    }
    if let Some(body) = body {
        req = req.body(body.to_string());
    }

    let resp = req
        .send()
        .await
        .map_err(|e| Error::Click(format!("couldn\u{27}t reach {url}: {e}")))?;

    let status = resp.status();
    if resp.content_length().is_some_and(|n| n > MAX_BYTES) {
        return Err(Error::Click(format!("{url} answered with too much to read")));
    }
    let text = resp
        .text()
        .await
        .map_err(|e| Error::Click(format!("couldn\u{27}t read what {url} said: {e}")))?;

    // The status is part of the answer, not a reason to hide it. A model that
    // can see `401 Unauthorized` knows to look for a token; one handed a bare
    // error message guesses.
    Ok(format!("{} {}\n\n{}", status.as_u16(), status.canonical_reason().unwrap_or(""), trim(&text)))
}

/// Why this request is not allowed, in words the model can act on.
fn refuse_request(
    verb: &str,
    url: &str,
    headers: &[(String, String)],
    granted: bool,
) -> Option<String> {
    if let Some(why) = refuse(url) {
        return Some(why);
    }
    if verb.is_empty() {
        return Some("no method given".into());
    }
    if !verb.chars().all(|c| c.is_ascii_uppercase()) {
        return Some(format!("{verb} is not an HTTP method"));
    }
    if !granted && !READING.contains(&verb) {
        return Some(format!(
            "{verb} could change something on the other end, and I have only been \
             allowed to read. Someone can grant that under \u{201c}Allowed to\u{201d} in the menu \
             bar -- until they do, I can {}",
            READING.join(" and ")
        ));
    }
    for (name, value) in headers {
        let lower = name.trim().to_ascii_lowercase();
        if lower.is_empty() {
            return Some("a header with no name".into());
        }
        if NOT_YOURS.contains(&lower.as_str()) {
            return Some(format!("{name} is not a header I will set for you"));
        }
        // A newline in either half splits one header into two, which is how a
        // request becomes two requests.
        if name.contains(['\r', '\n']) || value.contains(['\r', '\n']) {
            return Some(format!("{name} has a line break in it"));
        }
    }
    None
}

/// Strip a page down to what a person would actually read.
///
/// Falls back to the raw body rather than failing: a page that resists
/// extraction is still better read badly than not at all.
fn readable(html: &str, url: &str) -> String {
    use dom_smoothie::{Article, Config, Readability};
    let mut r = match Readability::new(html, Some(url), Some(Config::default())) {
        Ok(r) => r,
        Err(_) => return html.to_string(),
    };
    match r.parse() {
        Ok(Article {
            title,
            text_content,
            ..
        }) => {
            let body = text_content.trim();
            if body.is_empty() {
                html.to_string()
            } else {
                format!("{title}\n\n{body}")
            }
        }
        Err(_) => html.to_string(),
    }
}

/// Collapse the blank lines extraction leaves behind, and bound the result.
fn trim(text: &str) -> String {
    let mut out = String::with_capacity(text.len().min(MAX_CHARS));
    let mut blank = false;
    for line in text.lines() {
        let line = line.trim_end();
        if line.trim().is_empty() {
            if blank {
                continue;
            }
            blank = true;
        } else {
            blank = false;
        }
        // Truncated on a character boundary, not a byte one, and the line
        // itself is cut rather than merely ending the loop -- one page with no
        // newlines in it would otherwise sail past the limit whole.
        let room = MAX_CHARS.saturating_sub(out.len());
        if line.len() > room {
            let cut = line
                .char_indices()
                .map(|(i, _)| i)
                .take_while(|i| *i <= room)
                .last()
                .unwrap_or(0);
            out.push_str(&line[..cut]);
            out.push_str("\n… (truncated)\n");
            break;
        }
        out.push_str(line);
        out.push('\n');
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::{refuse_request as no};

    /// The line the grant draws: reading is always allowed, acting is not.
    #[test]
    fn acting_needs_a_grant_and_reading_does_not() {
        let url = "https://api.example.com/things";
        assert!(no("GET", url, &[], false).is_none());
        assert!(no("HEAD", url, &[], false).is_none());
        for verb in ["POST", "PUT", "PATCH", "DELETE"] {
            assert!(no(verb, url, &[], false).is_some(), "{verb} needed no grant");
            assert!(no(verb, url, &[], true).is_none(), "{verb} refused when granted");
        }
    }

    /// "Refused as clearly as the shell refuses" -- the plan's words. A refusal
    /// the model cannot act on is the same as a silent one.
    #[test]
    fn a_refusal_says_what_would_lift_it() {
        let why = no("POST", "https://api.example.com", &[], false).unwrap();
        assert!(why.contains("Allowed to"), "got: {why}");
        assert!(why.contains("GET"), "it should say what it still can do: {why}");
    }

    /// The URL rules are not weakened by having a method. A granted POST to a
    /// router on the home network is the request this most needs to refuse.
    #[test]
    fn the_network_rules_hold_however_wide_the_grant() {
        for url in [
            "http://localhost:5432/",
            "http://192.168.1.1/admin",
            "http://169.254.169.254/latest/meta-data/",
            "file:///etc/passwd",
        ] {
            assert!(no("POST", url, &[], true).is_some(), "{url} was allowed");
        }
    }

    #[test]
    fn headers_that_would_redirect_or_split_the_request_are_refused() {
        let url = "https://api.example.com";
        let host = [("Host".to_string(), "evil.example".to_string())];
        assert!(no("GET", url, &host, true).is_some(), "Host was accepted");

        let split = [("X-Thing".to_string(), "a\r\nX-Other: b".to_string())];
        assert!(no("GET", url, &split, true).is_some(), "a line break got through");

        let fine = [("Authorization".to_string(), "Bearer abc".to_string())];
        assert!(no("GET", url, &fine, false).is_none(), "an ordinary header was refused");
    }

    /// Against a real server, because the refusals above prove only what we
    /// refuse. Ignored by default: it needs a network.
    ///
    ///     cargo test --lib fetch -- --ignored --nocapture
    #[tokio::test]
    #[ignore = "needs the network"]
    async fn a_real_request_carries_its_method_headers_and_body() {
        let said = super::request(
            "post",
            "https://httpbin.org/post",
            &[("X-Nudge".into(), "hello".into())],
            Some(r#"{"a":1}"#),
            true,
        )
        .await
        .expect("the request failed");
        println!("{said}");
        assert!(said.starts_with("200 OK"), "status should lead: {said}");
        // httpbin echoes what it received, so this proves all three arrived.
        assert!(said.contains("\"a\": 1"), "the body did not arrive");
        assert!(said.contains("hello"), "the header did not arrive");

        // And the same call without the grant never leaves the machine.
        let refused = super::request("post", "https://httpbin.org/post", &[], None, false).await;
        println!("ungranted -> {refused:?}");
        assert!(refused.is_err());
    }

    #[test]
    fn a_method_has_to_look_like_one() {
        assert!(no("", "https://x.example", &[], true).is_some());
        assert!(no("get; rm -rf /", "https://x.example", &[], true).is_some());
    }

    use super::*;

    #[test]
    fn only_the_public_web() {
        for ok in [
            "https://example.com",
            "http://example.com/a/b?c=d",
            "https://wttr.in/Kathmandu",
            "https://sub.domain.example.co.uk/path",
        ] {
            assert_eq!(refuse(ok), None, "{ok} should be allowed");
        }
    }

    /// A model told to "check the printer" must not be able to reach one, and a
    /// development server on localhost is usually sitting on a real database.
    #[test]
    fn nothing_on_this_machine_or_this_network() {
        for bad in [
            "http://localhost:3000",
            "http://127.0.0.1/admin",
            "http://192.168.1.1",
            "http://10.0.0.5/",
            "http://172.16.0.1",
            "http://172.31.255.255",
            "http://printer.local",
            "http://db.internal/",
            "http://[::1]:8080",
            "http://169.254.169.254/latest/meta-data/",
        ] {
            assert!(refuse(bad).is_some(), "{bad} should be refused");
        }
        // 172.32 is public; only 172.16-31 is private, and getting that range
        // wrong in either direction is a real hole.
        assert_eq!(refuse("http://172.32.0.1"), None);
        assert_eq!(refuse("http://172.15.0.1"), None);
    }

    #[test]
    fn anything_that_is_not_a_web_page_is_refused() {
        for bad in [
            "file:///etc/passwd",
            "ftp://example.com",
            "javascript:alert(1)",
            "data:text/html,<h1>x</h1>",
            "",
            "example.com",
        ] {
            assert!(refuse(bad).is_some(), "{bad:?} should be refused");
        }
    }

    /// Credentials in a URL are a way to disguise the host: everything before
    /// the `@` is ignored by a browser, so `evil.com@localhost` goes to
    /// localhost.
    #[test]
    fn a_userinfo_prefix_does_not_hide_the_real_host() {
        assert!(refuse("http://example.com@localhost/").is_some());
        assert!(refuse("http://user:pass@127.0.0.1/").is_some());
    }

    /// The real thing: a live request, a real page, real extraction.
    ///
    /// Unit tests prove the rules; only this proves the crate does what it says
    /// on a page nobody wrote for us. Ignored by default because it needs the
    /// network.
    ///
    ///     cargo test reads_a_real_page -- --ignored --nocapture
    #[tokio::test]
    #[ignore]
    async fn reads_a_real_page() {
        // Plain text, so the answer is unmistakable if the request worked.
        let text = read("https://wttr.in/Kathmandu?format=3")
            .await
            .expect("fetch");
        eprintln!("wttr: {text}");
        assert!(text.to_lowercase().contains("kathmandu"), "got: {text}");

        // And an HTML page, which is where the extraction earns its place.
        let html = read("https://example.com").await.expect("fetch html");
        eprintln!("example.com: {html}");
        assert!(html.contains("Example Domain"), "got: {html}");
        assert!(!html.contains("<html"), "returned raw markup: {html}");
    }

    #[test]
    fn the_result_is_tidied_and_bounded() {
        let messy = "Title\n\n\n\n\nBody   \n\n\n\nEnd";
        assert_eq!(trim(messy), "Title\n\nBody\n\nEnd");

        let huge = "x".repeat(MAX_CHARS * 2);
        let out = trim(&huge);
        assert!(out.len() < MAX_CHARS + 200, "not bounded: {}", out.len());
        assert!(out.ends_with("(truncated)"));
    }

    #[test]
    fn html_becomes_the_words_a_person_would_read() {
        let page = r#"<html><head><title>Weather</title></head><body>
            <nav><a href="/">Home</a><a href="/x">Ads</a></nav>
            <article><h1>Kathmandu</h1><p>It is 24 degrees and raining lightly today in the
            Kathmandu valley, with cloud expected to clear by the evening.</p></article>
            <script>var tracking = 1;</script></body></html>"#;
        let out = readable(page, "https://example.com/weather");
        assert!(out.contains("24 degrees"), "lost the answer: {out}");
        assert!(!out.contains("var tracking"), "kept the script: {out}");
    }
}
