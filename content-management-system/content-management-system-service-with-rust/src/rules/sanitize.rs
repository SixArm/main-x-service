//! HTML sanitization at the write boundary (CMS-D5, CMS-R4).
//!
//! ## Why any HTML at all
//!
//! Content bodies are structured blocks, not markup, precisely so there
//! is nothing to smuggle. HTML enters at exactly one place in v1 — an
//! `embed` block's optional `html` payload (a third-party player, a
//! chart) — and (later) at bulk import. Both are sanitized **here, on
//! write**, so a stored document can never contain markup that was
//! merely trusted at render time.
//!
//! ## Why a parser rather than a regex or a hand-rolled stripper
//!
//! Sanitizing HTML by pattern-matching is the classic way to ship a
//! stored-XSS hole that passes its own tests: browsers recover from
//! malformed nesting, decode entities in odd places, and treat
//! `<svg><script>` and `<math><style>` as parsing contexts a naive
//! stripper never enters (mXSS). Correct sanitization requires a real
//! HTML5 tokenizer, so this module configures [`ammonia`] (html5ever)
//! with an explicit allow-list rather than reimplementing one. The
//! tests below pin the **policy** — what survives, what is removed —
//! not the parser, which is the part that would be pointless to
//! re-verify here.
//!
//! The same reasoning the family applied to S3 request signing
//! (`agents/share/bulk-import-export.md` §12): security-relevant
//! parsing code that looks finished but is unverified is worse than a
//! dependency.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

/// Elements permitted in sanitized HTML: structural and inline
/// formatting only. Nothing that loads or executes anything —
/// no `script`, `style`, `iframe`, `object`, `embed`, `form`, `svg`,
/// `math`, or event handlers (attributes are allow-listed too).
pub const ALLOWED_TAGS: &[&str] = &[
    "p",
    "br",
    "strong",
    "em",
    "b",
    "i",
    "u",
    "s",
    "code",
    "pre",
    "blockquote",
    "ul",
    "ol",
    "li",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "a",
    "figure",
    "figcaption",
    "table",
    "thead",
    "tbody",
    "tr",
    "th",
    "td",
    "span",
    "div",
];

/// URL schemes permitted on `a href`. Deliberately excludes
/// `javascript:` and `data:` — the two that turn a link into code.
pub const ALLOWED_SCHEMES: &[&str] = &["http", "https", "mailto"];

/// The process-wide sanitizer, built once from the allow-list.
fn builder() -> &'static ammonia::Builder<'static> {
    static BUILDER: OnceLock<ammonia::Builder<'static>> = OnceLock::new();
    BUILDER.get_or_init(|| {
        let mut builder = ammonia::Builder::default();
        builder.tags(ALLOWED_TAGS.iter().copied().collect::<HashSet<_>>());
        builder.url_schemes(ALLOWED_SCHEMES.iter().copied().collect::<HashSet<_>>());
        // Attributes are allow-listed per tag: only a link's target and
        // a cell's spans. No `style` (CSS is an execution surface in
        // enough contexts to not be worth the argument), no `class`
        // (the channel owns presentation — CMS-D6), no `id` (it would
        // let content collide with the host page's anchors).
        let mut attributes: HashMap<&str, HashSet<&str>> = HashMap::new();
        attributes.insert("a", ["href", "title"].into_iter().collect());
        attributes.insert("td", ["colspan", "rowspan"].into_iter().collect());
        attributes.insert("th", ["colspan", "rowspan", "scope"].into_iter().collect());
        builder.tag_attributes(attributes);
        builder.generic_attributes(HashSet::new());
        // Every surviving link is untrusted third-party markup, so it
        // leaves with `rel="noopener noreferrer"` and cannot reach back
        // through `window.opener`.
        builder.link_rel(Some("noopener noreferrer"));
        builder
    })
}

/// Sanitize `html` against the allow-list, returning safe markup.
///
/// Disallowed **elements** are removed but their text content is kept
/// (so a `<div style=…>` becomes its text rather than vanishing);
/// disallowed **attributes**, URL schemes, and anything script-bearing
/// are dropped entirely.
#[must_use]
pub fn sanitize_html(html: &str) -> String {
    builder().clean(html).to_string()
}

/// Whether `html` survives sanitization unchanged — i.e. it was
/// already safe. Used to tell a caller that their markup *was* altered,
/// rather than silently storing something different from what they sent.
#[must_use]
pub fn is_clean(html: &str) -> bool {
    sanitize_html(html) == html
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ordinary formatting survives intact.
    #[test]
    fn safe_markup_passes_through() {
        for html in [
            "<p>Hello <strong>world</strong></p>",
            "<ul><li>one</li><li>two</li></ul>",
            "<blockquote><p>Quoted</p></blockquote>",
        ] {
            assert_eq!(sanitize_html(html), html, "{html} should survive");
            assert!(is_clean(html));
        }
    }

    /// The hostile corpus: every one of these is a way people have
    /// actually smuggled script past a naive stripper.
    #[test]
    fn script_bearing_markup_never_survives() {
        let hostile = [
            r"<script>alert(1)</script>",
            r"<img src=x onerror=alert(1)>",
            r#"<a href="javascript:alert(1)">click</a>"#,
            r#"<a href="JaVaScRiPt:alert(1)">click</a>"#,
            r#"<a href="data:text/html,<script>alert(1)</script>">click</a>"#,
            r"<svg><script>alert(1)</script></svg>",
            r"<math><style><img src=x onerror=alert(1)></style></math>",
            r#"<iframe src="https://evil.test"></iframe>"#,
            r#"<object data="evil.swf"></object>"#,
            r#"<form action="https://evil.test"><input name=a></form>"#,
            r"<body onload=alert(1)>",
            r#"<p onclick="alert(1)">text</p>"#,
            r"<style>body{background:url(javascript:alert(1))}</style>",
            r#"<p style="background:url(javascript:alert(1))">text</p>"#,
            // Malformed nesting: the case a regex-based stripper fails.
            r"<p><scr<script>ipt>alert(1)</script></p>",
            r#"<<a href="javascript:alert(1)">a"#,
        ];
        for html in hostile {
            let clean = sanitize_html(html);
            let lower = clean.to_ascii_lowercase();
            assert!(
                !lower.contains("<script"),
                "script survived {html:?} => {clean:?}"
            );
            assert!(
                !lower.contains("javascript:"),
                "js: URL survived {html:?} => {clean:?}"
            );
            assert!(
                !lower.contains("onerror"),
                "handler survived {html:?} => {clean:?}"
            );
            assert!(
                !lower.contains("onclick"),
                "handler survived {html:?} => {clean:?}"
            );
            assert!(
                !lower.contains("onload"),
                "handler survived {html:?} => {clean:?}"
            );
            assert!(
                !lower.contains("<iframe"),
                "iframe survived {html:?} => {clean:?}"
            );
            assert!(
                !lower.contains("<style"),
                "style survived {html:?} => {clean:?}"
            );
            assert!(
                !lower.contains("<form"),
                "form survived {html:?} => {clean:?}"
            );
            assert!(
                !lower.contains("<svg"),
                "svg survived {html:?} => {clean:?}"
            );
            assert!(!is_clean(html), "{html:?} should be reported as altered");
        }
    }

    /// A safe link keeps its target and gains `rel`, so untrusted
    /// third-party markup cannot reach back through `window.opener`.
    #[test]
    fn links_keep_safe_targets_and_gain_rel() {
        let clean = sanitize_html(r#"<a href="https://example.test">x</a>"#);
        assert!(clean.contains(r#"href="https://example.test""#));
        assert!(clean.contains("noopener"));
        assert!(clean.contains("noreferrer"));
        // mailto is allowed; ftp and data are not.
        assert!(sanitize_html(r#"<a href="mailto:a@b.test">x</a>"#).contains("mailto:"));
        assert!(!sanitize_html(r#"<a href="ftp://example.test">x</a>"#).contains("ftp:"));
    }

    /// A disallowed element loses its tag but keeps its words: content
    /// is not silently destroyed just because its wrapper was refused.
    #[test]
    fn disallowed_wrappers_keep_their_text() {
        let clean = sanitize_html(r"<marquee>important notice</marquee>");
        assert!(clean.contains("important notice"));
        assert!(!clean.contains("marquee"));
    }

    /// Presentation attributes are dropped: the channel owns styling
    /// (CMS-D6), and `class`/`id` from content could collide with the
    /// host page.
    #[test]
    fn presentation_attributes_are_dropped() {
        let clean = sanitize_html(r#"<p class="lede" id="top" style="color:red">x</p>"#);
        assert_eq!(clean, "<p>x</p>");
    }

    /// Sanitizing is idempotent — a second pass changes nothing, so a
    /// re-save cannot slowly rewrite stored content.
    #[test]
    fn sanitizing_is_idempotent() {
        for html in [
            r#"<p>text <a href="https://a.test">link</a></p>"#,
            r"<img src=x onerror=alert(1)><p>after</p>",
            r#"<div class="x"><marquee>y</marquee></div>"#,
        ] {
            let once = sanitize_html(html);
            assert_eq!(sanitize_html(&once), once, "not idempotent for {html:?}");
        }
    }

    /// Empty and plain-text inputs are handled without panicking or
    /// inventing markup.
    #[test]
    fn plain_text_is_untouched() {
        assert_eq!(sanitize_html(""), "");
        assert_eq!(sanitize_html("just words"), "just words");
        // Angle brackets in prose are escaped, not executed.
        assert!(!sanitize_html("a < b && c > d").contains("<b"));
    }
}
