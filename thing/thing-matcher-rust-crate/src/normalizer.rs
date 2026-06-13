//! Text normalisation for `Thing` matching.
//!
//! Most matching accuracy gains come from **standardising the input** before
//! scoring, not from cleverer similarity algorithms. This module exposes the
//! canonical transformations the matching engine applies to names, free-form
//! text, URLs, and phonetic codes.
//!
//! All transformations are **idempotent**: `f(f(x)) == f(x)`. They are also
//! **deterministic** and allocate at most a single new `String`.
//!
//! ## Quick examples
//!
//! ```
//! use thing_matcher::Normalizer;
//!
//! // Names: lowercase, drop diacritics, drop ASCII punctuation, collapse spaces.
//! assert_eq!(Normalizer::normalize_name("  O'Brien  "), "obrien");
//! assert_eq!(Normalizer::normalize_name("Siân"),         "sian");
//!
//! // Free-form text: lowercase, NFKD, collapse whitespace; keep punctuation
//! // (so descriptions remain readable).
//! assert_eq!(
//!     Normalizer::normalize_text("  The   Eiffel Tower.  "),
//!     "the eiffel tower.",
//! );
//!
//! // URLs: lowercase scheme + host, drop trailing slash on the path root.
//! assert_eq!(
//!     Normalizer::normalize_url("HTTPS://Example.ORG/"),
//!     "https://example.org",
//! );
//! ```
//!
//! ## What this module deliberately does *not* do
//!
//! - It does not handle non-ASCII punctuation such as the curly apostrophe
//!   `’` (U+2019). Upstream code should convert those to ASCII first.
//! - It does not perform DNS-aware URL normalisation, percent-encoding
//!   canonicalisation, or punycode decoding.

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

/// Stateless namespace for text normalisation routines.
///
/// `Normalizer` is a unit type with no fields; every method is associated.
/// It is held as a struct rather than a free function module purely so the
/// public API has a single, discoverable entry point.
///
/// ```
/// use thing_matcher::Normalizer;
///
/// let canonical = Normalizer::normalize_name("José-María");
/// assert_eq!(canonical, "josemaria");
/// ```
pub struct Normalizer;

impl Normalizer {
    /// Normalise a name for comparison.
    ///
    /// Steps, in order:
    ///
    /// 1. Decompose to Unicode NFKD form (`é` → `e` + combining acute).
    /// 2. Drop combining marks (diacritics).
    /// 3. Drop ASCII punctuation (apostrophes, hyphens, full stops, …).
    /// 4. Lowercase.
    /// 5. Collapse consecutive whitespace to single ASCII spaces; trim ends.
    ///
    /// The result is suitable for direct equality comparison or for feeding
    /// into a string-similarity scorer.
    ///
    /// # Examples
    ///
    /// Whitespace is collapsed and trimmed:
    ///
    /// ```
    /// use thing_matcher::Normalizer;
    /// assert_eq!(Normalizer::normalize_name("  John  Smith  "), "john smith");
    /// ```
    ///
    /// Apostrophes and hyphens are stripped:
    ///
    /// ```
    /// # use thing_matcher::Normalizer;
    /// assert_eq!(Normalizer::normalize_name("O'Brien"),    "obrien");
    /// assert_eq!(Normalizer::normalize_name("MARY-JANE"),  "maryjane");
    /// ```
    ///
    /// Diacritics are removed:
    ///
    /// ```
    /// # use thing_matcher::Normalizer;
    /// assert_eq!(Normalizer::normalize_name("Siân"),    "sian");
    /// assert_eq!(Normalizer::normalize_name("café"),    "cafe");
    /// // Letters with an integral stroke do not decompose under NFKD, so
    /// // they pass through (lowercased), while the combining acute on `ó`
    /// // and `ź` is stripped:
    /// assert_eq!(Normalizer::normalize_name("Łódź"),    "łodz");
    /// ```
    pub fn normalize_name(name: &str) -> String {
        let mut out = String::with_capacity(name.len());
        for ch in name.nfkd() {
            // Skip combining marks (Unicode categories Mn / Mc / Me).
            if is_combining_mark(ch) {
                continue;
            }
            if ch.is_ascii_punctuation() {
                continue;
            }
            for lc in ch.to_lowercase() {
                out.push(lc);
            }
        }
        collapse_whitespace(&out)
    }

    /// Normalise free-form text (descriptions, etc.) for similarity scoring.
    ///
    /// Like [`Normalizer::normalize_name`], but keeps ASCII punctuation —
    /// punctuation carries information in longer text (sentence boundaries,
    /// abbreviations) that should not be discarded.
    ///
    /// Steps, in order:
    ///
    /// 1. Decompose to Unicode NFKD form.
    /// 2. Drop combining marks (diacritics).
    /// 3. Lowercase.
    /// 4. Collapse consecutive whitespace to single ASCII spaces; trim ends.
    ///
    /// # Examples
    ///
    /// ```
    /// use thing_matcher::Normalizer;
    /// assert_eq!(
    ///     Normalizer::normalize_text("  The Eiffel Tower, in Paris.  "),
    ///     "the eiffel tower, in paris.",
    /// );
    /// assert_eq!(
    ///     Normalizer::normalize_text("café au lait"),
    ///     "cafe au lait",
    /// );
    /// ```
    pub fn normalize_text(text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        for ch in text.nfkd() {
            if is_combining_mark(ch) {
                continue;
            }
            for lc in ch.to_lowercase() {
                out.push(lc);
            }
        }
        collapse_whitespace(&out)
    }

    /// Normalise a URL for equality comparison.
    ///
    /// The transformation is **lossless enough for matching** but **not a
    /// full URL canonicalisation**:
    ///
    /// 1. Trim surrounding whitespace.
    /// 2. Lowercase the scheme and host portions (`HTTPS://Example.ORG` →
    ///    `https://example.org`). The path is left case-sensitive.
    /// 3. Drop a trailing slash from a root path (`https://x.org/` →
    ///    `https://x.org`). Non-root trailing slashes are kept, because
    ///    `/foo` and `/foo/` are legitimately different on many servers.
    /// 4. Drop a `#fragment` suffix — fragments do not travel over HTTP
    ///    and never identify a different resource.
    ///
    /// No percent-encoding canonicalisation is attempted; callers that
    /// need strict canonical URLs should pre-process the input.
    ///
    /// # Examples
    ///
    /// ```
    /// use thing_matcher::Normalizer;
    /// assert_eq!(
    ///     Normalizer::normalize_url("HTTPS://Example.ORG/"),
    ///     "https://example.org",
    /// );
    /// assert_eq!(
    ///     Normalizer::normalize_url("  https://EXAMPLE.org/foo  "),
    ///     "https://example.org/foo",
    /// );
    /// assert_eq!(
    ///     Normalizer::normalize_url("https://example.org/foo/#bar"),
    ///     "https://example.org/foo/",
    /// );
    /// ```
    ///
    /// Strings that are not URL-shaped are returned trimmed + lowercased
    /// so they remain comparable as opaque identifiers:
    ///
    /// ```
    /// # use thing_matcher::Normalizer;
    /// assert_eq!(Normalizer::normalize_url("  URN:ISBN:0451450523  "), "urn:isbn:0451450523");
    /// ```
    pub fn normalize_url(url: &str) -> String {
        let trimmed = url.trim();
        // Drop fragment, if present. Re-trim the trailing end: removing the
        // `#fragment` can expose whitespace that sat just before the `#`
        // (e.g. `"x \u{2000}#frag"`), which the initial `trim` could not
        // reach. Without this, a second pass would strip that whitespace and
        // `normalize_url` would not be idempotent.
        let no_frag = match trimmed.find('#') {
            Some(idx) => trimmed[..idx].trim_end(),
            None => trimmed,
        };

        // Locate scheme delimiter.
        let (scheme, after_scheme) = match no_frag.find("://") {
            Some(idx) => (&no_frag[..idx], Some(&no_frag[idx + 3..])),
            None => (no_frag, None),
        };

        // No scheme — fall back to a trimmed lowercase opaque form. Useful
        // for `urn:` / `mailto:` / `tel:` style identifiers.
        let Some(rest) = after_scheme else {
            return no_frag.to_ascii_lowercase();
        };

        // Split host from path.
        let (host, path) = match rest.find('/') {
            Some(idx) => (&rest[..idx], &rest[idx..]),
            None => (rest, ""),
        };

        let mut out = String::with_capacity(no_frag.len());
        out.push_str(&scheme.to_ascii_lowercase());
        out.push_str("://");
        out.push_str(&host.to_ascii_lowercase());

        // Drop a trailing slash only when the path *is* the root.
        if !(path.is_empty() || path == "/") {
            out.push_str(path);
        }
        out
    }

    /// Soundex-like phonetic code for an ASCII-ish name, used as a coarse
    /// blocking key and as the gate for the phonetic-bonus in the matcher.
    ///
    /// Implementation note: delegates to the `soundex` crate after first
    /// applying [`Normalizer::normalize_name`]. Returns an empty string
    /// when the input is empty or normalises to an empty string.
    ///
    /// # Examples
    ///
    /// ```
    /// use thing_matcher::Normalizer;
    /// let a = Normalizer::phonetic_code("Stephen");
    /// let b = Normalizer::phonetic_code("Steven");
    /// assert!(!a.is_empty());
    /// assert_eq!(a, b);
    /// ```
    pub fn phonetic_code(name: &str) -> String {
        let normalised = Self::normalize_name(name);
        if normalised.is_empty() {
            return String::new();
        }
        // The `soundex` crate's `american_soundex` is infallible for any
        // ASCII input. Strip non-ASCII bytes before handing it over.
        let ascii: String = normalised.chars().filter(char::is_ascii).collect();
        if ascii.is_empty() {
            return String::new();
        }
        soundex::american_soundex(&ascii)
    }
}

/// Collapse consecutive whitespace into single ASCII spaces and trim ends.
fn collapse_whitespace(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = true; // start of string = no leading spaces
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    if out.ends_with(' ') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- normalize_name ----------

    #[test]
    fn normalize_name_lowercases_and_trims() {
        assert_eq!(Normalizer::normalize_name("  HELLO  "), "hello");
    }

    #[test]
    fn normalize_name_collapses_internal_whitespace() {
        assert_eq!(Normalizer::normalize_name("a  \t  b\nc"), "a b c");
    }

    #[test]
    fn normalize_name_drops_punctuation() {
        assert_eq!(Normalizer::normalize_name("O'Brien"), "obrien");
        assert_eq!(Normalizer::normalize_name("Mary-Jane!"), "maryjane");
    }

    #[test]
    fn normalize_name_drops_diacritics() {
        assert_eq!(Normalizer::normalize_name("Siân"), "sian");
        assert_eq!(Normalizer::normalize_name("café"), "cafe");
        assert_eq!(Normalizer::normalize_name("Zoë"), "zoe");
    }

    #[test]
    fn normalize_name_is_idempotent() {
        let cases = ["hello", "O'Brien", " café au lait ", "JOSÉ-MARÍA"];
        for c in cases {
            let once = Normalizer::normalize_name(c);
            let twice = Normalizer::normalize_name(&once);
            assert_eq!(once, twice, "non-idempotent for {c:?}");
        }
    }

    #[test]
    fn normalize_name_empty_returns_empty() {
        assert!(Normalizer::normalize_name("").is_empty());
        assert!(Normalizer::normalize_name("    ").is_empty());
    }

    // ---------- normalize_text ----------

    #[test]
    fn normalize_text_preserves_punctuation() {
        assert_eq!(Normalizer::normalize_text("Hello, World!"), "hello, world!");
    }

    #[test]
    fn normalize_text_drops_diacritics() {
        assert_eq!(Normalizer::normalize_text("Café au lait."), "cafe au lait.");
    }

    #[test]
    fn normalize_text_is_idempotent() {
        let cases = [
            "The Eiffel Tower, in Paris.",
            "  multi    space   ",
            "Plain.",
        ];
        for c in cases {
            let once = Normalizer::normalize_text(c);
            let twice = Normalizer::normalize_text(&once);
            assert_eq!(once, twice, "non-idempotent for {c:?}");
        }
    }

    // ---------- normalize_url ----------

    #[test]
    fn normalize_url_lowercases_scheme_and_host() {
        assert_eq!(
            Normalizer::normalize_url("HTTPS://Example.ORG/foo"),
            "https://example.org/foo",
        );
    }

    #[test]
    fn normalize_url_drops_root_trailing_slash() {
        assert_eq!(
            Normalizer::normalize_url("https://example.org/"),
            "https://example.org",
        );
    }

    #[test]
    fn normalize_url_keeps_subpath_trailing_slash() {
        assert_eq!(
            Normalizer::normalize_url("https://example.org/foo/"),
            "https://example.org/foo/",
        );
    }

    #[test]
    fn normalize_url_drops_fragment() {
        assert_eq!(
            Normalizer::normalize_url("https://example.org/foo#bar"),
            "https://example.org/foo",
        );
    }

    #[test]
    fn normalize_url_handles_opaque_uri() {
        assert_eq!(
            Normalizer::normalize_url("URN:ISBN:0451450523"),
            "urn:isbn:0451450523",
        );
    }

    #[test]
    fn normalize_url_is_idempotent() {
        let cases = [
            "https://example.org/",
            "HTTPS://EXAMPLE.org/foo#frag",
            "urn:isbn:123",
            // Whitespace sitting just before a `#fragment`: dropping the
            // fragment exposes it, so it must be re-trimmed (regression).
            "\u{1F300}\u{2000}#",
            "http://h/p \u{2000}#x",
        ];
        for c in cases {
            let once = Normalizer::normalize_url(c);
            let twice = Normalizer::normalize_url(&once);
            assert_eq!(once, twice, "non-idempotent for {c:?}");
        }
    }

    #[test]
    fn normalize_url_retrims_after_fragment_removal() {
        assert_eq!(Normalizer::normalize_url("http://h/p \u{2000}#x"), "http://h/p");
        assert_eq!(Normalizer::normalize_url("\u{1F300}\u{2000}#frag"), "\u{1F300}");
    }

    // ---------- phonetic_code ----------

    #[test]
    fn phonetic_code_matches_homophones() {
        assert_eq!(
            Normalizer::phonetic_code("Stephen"),
            Normalizer::phonetic_code("Steven"),
        );
    }

    #[test]
    fn phonetic_code_distinct_for_unrelated_names() {
        assert_ne!(
            Normalizer::phonetic_code("Alice"),
            Normalizer::phonetic_code("Zachary"),
        );
    }

    #[test]
    fn phonetic_code_empty_for_empty_input() {
        assert!(Normalizer::phonetic_code("").is_empty());
        assert!(Normalizer::phonetic_code("   ").is_empty());
    }
}
