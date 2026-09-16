//! Reading a document Nudge was pointed at.
//!
//! `read` handles text files, and everything people actually want summarised is
//! a PDF: a contract, an invoice, a statement, the thing somebody was sent this
//! morning. Pointed at one, Nudge got bytes that are not text and said so.
//!
//! ## Through PDFKit, not a parser
//!
//! macOS already reads PDFs -- Preview, Quick Look and Spotlight all do it with
//! the same framework -- so the text is one framework call away. A Rust PDF
//! parser would be a second implementation of something already on the machine,
//! with its own opinion about broken files and its own CVEs.
//!
//! The alternative considered and rejected: shelling out to `pdftotext`. It is
//! not on a Mac unless somebody installed Homebrew and then installed poppler,
//! so a feature built on it works here and fails for everybody else — which is
//! the worst way to find out.
//!
//! ## What it will not do
//!
//! A scanned page has no text in it, only a picture of text. PDFKit returns
//! nothing and this says so plainly rather than returning an empty string that
//! reads as an empty document. Making that work means optical recognition,
//! which is a different feature and a much larger one.
use crate::error::{Error, Result};
use std::path::Path;

/// Roughly a hundred pages of prose.
///
/// The same reasoning as every other clip here: a whole book in the history is
/// a whole book in every later turn, and a model that needs page ninety can be
/// pointed at the page.
const MOST: usize = 120_000;

/// Does this look like something only this module can read?
pub fn is_paper(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("pdf")
    )
}

/// The text of a document, or a sentence explaining why there is none.
#[cfg(target_os = "macos")]
pub fn read(path: &Path) -> Result<String> {
    use objc2::AnyThread;
    use objc2_foundation::{NSString, NSURL};
    use objc2_pdf_kit::PDFDocument;

    if !path.is_file() {
        return Err(Error::Click(format!("{} is not a file", path.display())));
    }

    let text = unsafe {
        let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
        let Some(doc) = PDFDocument::initWithURL(PDFDocument::alloc(), &url) else {
            return Err(Error::Click(format!(
                "{} could not be opened as a PDF -- it may be damaged or password-protected",
                path.display()
            )));
        };
        let pages = doc.pageCount();
        match doc.string() {
            Some(s) => (s.to_string(), pages),
            None => (String::new(), pages),
        }
    };
    let (text, pages) = text;

    if text.trim().is_empty() {
        // Said rather than returned as an empty document, which reads as "there
        // was nothing in it" and sends the model off answering from nothing.
        return Err(Error::Click(format!(
            "{} has {pages} page(s) but no text in it -- it is probably a scan, \
             which would need optical recognition Nudge does not do",
            path.display()
        )));
    }

    Ok(clip(&text, pages))
}

#[cfg(not(target_os = "macos"))]
pub fn read(path: &Path) -> Result<String> {
    Err(Error::Click(format!(
        "reading {} needs macOS",
        path.display()
    )))
}

/// Bound it, and say what was left out rather than stopping mid-sentence.
fn clip(text: &str, pages: usize) -> String {
    let flat = text.trim();
    if flat.chars().count() <= MOST {
        return format!("{pages} page(s):\n{flat}");
    }
    let kept: String = flat.chars().take(MOST).collect();
    format!(
        "{pages} page(s), showing the first {MOST} characters:\n{kept}\n\
         … (the rest is not shown — ask for a specific part if you need it)"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_pdf_is_paper() {
        assert!(is_paper(Path::new("/a/contract.pdf")));
        assert!(is_paper(Path::new("/a/CONTRACT.PDF")));
        assert!(!is_paper(Path::new("/a/notes.txt")));
        assert!(!is_paper(Path::new("/a/pdf")));
        assert!(!is_paper(Path::new("/a/thing.pdfx")));
    }

    #[test]
    fn a_missing_file_says_so_rather_than_crashing() {
        let e = read(Path::new("/nowhere/at/all.pdf"))
            .unwrap_err()
            .to_string();
        assert!(e.contains("not a file"), "{e}");
    }

    /// Something that is not a PDF at all must not come back as an empty one.
    #[test]
    fn a_file_that_is_not_a_pdf_is_refused_rather_than_read_as_blank() {
        let path = std::env::temp_dir().join(format!("nudge-notapdf-{}.pdf", std::process::id()));
        std::fs::write(&path, b"this is plainly not a pdf").unwrap();
        let e = read(&path).unwrap_err().to_string();
        assert!(
            e.contains("could not be opened") || e.contains("no text"),
            "{e}"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_long_document_says_what_it_left_out() {
        let out = clip(&"word ".repeat(MOST), 300);
        assert!(out.starts_with("300 page(s), showing the first"));
        assert!(
            out.ends_with("if you need it)"),
            "{}",
            &out[out.len() - 80..]
        );
    }

    #[test]
    fn a_short_one_is_given_whole() {
        let out = clip("Invoice 4471", 1);
        assert_eq!(out, "1 page(s):\nInvoice 4471");
    }
}
