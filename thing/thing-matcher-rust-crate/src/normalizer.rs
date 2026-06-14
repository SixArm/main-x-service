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
        // Pre-size for the common case where output length ≈ input length.
        let mut out = String::with_capacity(name.len());
        // Iterate the NFKD decomposition so that pre-composed characters
        // such as `é` arrive as a base letter followed by a combining mark.
        for ch in name.nfkd() {
            // Skip combining marks (Unicode categories Mn / Mc / Me). This
            // is the diacritic-stripping step: with `é` already split into
            // `e` + acute, dropping the acute yields a plain `e`.
            if is_combining_mark(ch) {
                continue;
            }
            // Names: drop ASCII punctuation so `O'Brien` and `OBrien`, or
            // `Mary-Jane` and `MaryJane`, collapse to the same key.
            if ch.is_ascii_punctuation() {
                continue;
            }
            // Lowercase. `to_lowercase` can yield multiple chars (e.g. the
            // German ß) so push each one.
            for lc in ch.to_lowercase() {
                out.push(lc);
            }
        }
        // Final whitespace pass: collapse runs to single spaces and trim.
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
            // Strip diacritics, same as `normalize_name`.
            if is_combining_mark(ch) {
                continue;
            }
            // NOTE: unlike `normalize_name`, ASCII punctuation is *kept* —
            // sentence structure carries meaning in free-form descriptions.
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

        // Locate the `://` scheme delimiter that separates a hierarchical
        // URL (`https://host/path`) from an opaque URI (`urn:isbn:...`).
        let (scheme, after_scheme) = match no_frag.find("://") {
            // `idx + 3` skips past the three-byte `://` literal.
            Some(idx) => (&no_frag[..idx], Some(&no_frag[idx + 3..])),
            None => (no_frag, None),
        };

        // No scheme — fall back to a trimmed lowercase opaque form. Useful
        // for `urn:` / `mailto:` / `tel:` style identifiers. We lowercase
        // the whole thing because opaque schemes have no case-sensitive
        // path component to preserve.
        let Some(rest) = after_scheme else {
            return no_frag.to_ascii_lowercase();
        };

        // Split the authority (host) from the path at the first `/`. When
        // there is no `/`, the whole remainder is the host and the path is
        // empty.
        let (host, path) = match rest.find('/') {
            Some(idx) => (&rest[..idx], &rest[idx..]),
            None => (rest, ""),
        };

        let mut out = String::with_capacity(no_frag.len());
        // Scheme and host are case-insensitive per RFC 3986, so lowercase
        // them; the path is left as-is because many servers treat it as
        // case-sensitive.
        out.push_str(&scheme.to_ascii_lowercase());
        out.push_str("://");
        out.push_str(&host.to_ascii_lowercase());

        // Drop a trailing slash only when the path *is* the root (empty or
        // a lone `/`). A trailing slash on a deeper path (`/foo/`) is kept
        // because `/foo` and `/foo/` can legitimately differ.
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
        // Normalise first so diacritics / punctuation / case do not perturb
        // the phonetic code: `José` and `Jose` must produce the same code.
        let normalised = Self::normalize_name(name);
        if normalised.is_empty() {
            return String::new();
        }
        // The `soundex` crate's `american_soundex` is infallible for any
        // ASCII input but can misbehave on non-ASCII. Strip non-ASCII chars
        // before handing it over; if nothing ASCII remains, there is no
        // meaningful Soundex code to compute.
        let ascii: String = normalised.chars().filter(char::is_ascii).collect();
        if ascii.is_empty() {
            return String::new();
        }
        soundex::american_soundex(&ascii)
    }
}

/// Collapse consecutive whitespace into single ASCII spaces and trim ends.
///
/// Any run of one or more whitespace characters (of any kind — tabs,
/// newlines, Unicode spaces) becomes exactly one ASCII space, and leading /
/// trailing whitespace is removed. This is the shared final step of
/// [`Normalizer::normalize_name`] and [`Normalizer::normalize_text`].
fn collapse_whitespace(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    // `prev_space` tracks whether the previous emitted position was a space
    // boundary. Seeding it to `true` suppresses any leading whitespace:
    // the start of the string is treated as if a space had just been seen.
    let mut prev_space = true; // start of string = no leading spaces
    for ch in s.chars() {
        if ch.is_whitespace() {
            // Emit at most one space per run: only the first whitespace
            // char of a run pushes a space; subsequent ones are swallowed.
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    // A run of trailing whitespace will have pushed exactly one space; pop
    // it so the result is trimmed on the right.
    if out.ends_with(' ') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- normalize_name ----------

    /// Pins lowercasing and end-trimming on names.
    #[test]
    fn normalize_name_lowercases_and_trims() {
        assert_eq!(Normalizer::normalize_name("  HELLO  "), "hello");
    }

    /// Pins that mixed internal whitespace (spaces, tabs, newlines)
    /// collapses to single ASCII spaces.
    #[test]
    fn normalize_name_collapses_internal_whitespace() {
        assert_eq!(Normalizer::normalize_name("a  \t  b\nc"), "a b c");
    }

    /// Pins ASCII-punctuation removal for names — apostrophes, hyphens, and
    /// exclamation marks all vanish.
    #[test]
    fn normalize_name_drops_punctuation() {
        assert_eq!(Normalizer::normalize_name("O'Brien"), "obrien");
        assert_eq!(Normalizer::normalize_name("Mary-Jane!"), "maryjane");
    }

    /// Pins diacritic stripping via NFKD decomposition (`â`→`a`, `é`→`e`,
    /// `ë`→`e`).
    #[test]
    fn normalize_name_drops_diacritics() {
        assert_eq!(Normalizer::normalize_name("Siân"), "sian");
        assert_eq!(Normalizer::normalize_name("café"), "cafe");
        assert_eq!(Normalizer::normalize_name("Zoë"), "zoe");
    }

    /// Pins the idempotence invariant for names: normalising an already
    /// normalised string is a no-op.
    #[test]
    fn normalize_name_is_idempotent() {
        let cases = ["hello", "O'Brien", " café au lait ", "JOSÉ-MARÍA"];
        for c in cases {
            let once = Normalizer::normalize_name(c);
            let twice = Normalizer::normalize_name(&once);
            assert_eq!(once, twice, "non-idempotent for {c:?}");
        }
    }

    /// Pins that empty and whitespace-only inputs normalise to the empty
    /// string (used downstream to skip a thing with no usable name).
    #[test]
    fn normalize_name_empty_returns_empty() {
        assert!(Normalizer::normalize_name("").is_empty());
        assert!(Normalizer::normalize_name("    ").is_empty());
    }

    // ---------- normalize_text ----------

    /// Pins the key difference from `normalize_name`: free-form text *keeps*
    /// punctuation (comma, exclamation mark) while still lowercasing.
    #[test]
    fn normalize_text_preserves_punctuation() {
        assert_eq!(Normalizer::normalize_text("Hello, World!"), "hello, world!");
    }

    /// Pins that free-form text still strips diacritics even though it keeps
    /// punctuation.
    #[test]
    fn normalize_text_drops_diacritics() {
        assert_eq!(Normalizer::normalize_text("Café au lait."), "cafe au lait.");
    }

    /// Pins the idempotence invariant for free-form text.
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

    /// Pins that scheme and host lowercase while the path keeps its case.
    #[test]
    fn normalize_url_lowercases_scheme_and_host() {
        assert_eq!(
            Normalizer::normalize_url("HTTPS://Example.ORG/foo"),
            "https://example.org/foo",
        );
    }

    /// Pins that a trailing slash on the *root* path is dropped.
    #[test]
    fn normalize_url_drops_root_trailing_slash() {
        assert_eq!(
            Normalizer::normalize_url("https://example.org/"),
            "https://example.org",
        );
    }

    /// Pins the counterpart rule: a trailing slash on a *sub*-path is
    /// preserved, because `/foo` and `/foo/` can differ on real servers.
    #[test]
    fn normalize_url_keeps_subpath_trailing_slash() {
        assert_eq!(
            Normalizer::normalize_url("https://example.org/foo/"),
            "https://example.org/foo/",
        );
    }

    /// Pins fragment removal — the `#bar` suffix is dropped.
    #[test]
    fn normalize_url_drops_fragment() {
        assert_eq!(
            Normalizer::normalize_url("https://example.org/foo#bar"),
            "https://example.org/foo",
        );
    }

    /// Pins the opaque-URI fallback: a scheme without `://` (here a URN) is
    /// returned trimmed and fully lowercased.
    #[test]
    fn normalize_url_handles_opaque_uri() {
        assert_eq!(
            Normalizer::normalize_url("URN:ISBN:0451450523"),
            "urn:isbn:0451450523",
        );
    }

    /// Pins the idempotence invariant for URLs, including the tricky
    /// whitespace-before-fragment regression cases.
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

    /// Pins the subtle re-trim step: removing a `#fragment` can expose
    /// whitespace that sat just before the `#` (and that the initial `trim`
    /// could not reach). The `trim_end` after the cut is what keeps
    /// `normalize_url` idempotent on these inputs.
    #[test]
    fn normalize_url_retrims_after_fragment_removal() {
        assert_eq!(Normalizer::normalize_url("http://h/p \u{2000}#x"), "http://h/p");
        assert_eq!(Normalizer::normalize_url("\u{1F300}\u{2000}#frag"), "\u{1F300}");
    }

    // ---------- phonetic_code ----------

    /// Pins that homophones (`Stephen` / `Steven`) share a Soundex code —
    /// the basis for the matcher's optional phonetic bonus.
    #[test]
    fn phonetic_code_matches_homophones() {
        assert_eq!(
            Normalizer::phonetic_code("Stephen"),
            Normalizer::phonetic_code("Steven"),
        );
    }

    /// Pins that phonetically unrelated names get distinct codes, so the
    /// bonus does not fire for arbitrary pairs.
    #[test]
    fn phonetic_code_distinct_for_unrelated_names() {
        assert_ne!(
            Normalizer::phonetic_code("Alice"),
            Normalizer::phonetic_code("Zachary"),
        );
    }

    /// Pins the empty-input contract: no name → empty code (never a panic).
    #[test]
    fn phonetic_code_empty_for_empty_input() {
        assert!(Normalizer::phonetic_code("").is_empty());
        assert!(Normalizer::phonetic_code("   ").is_empty());
    }
}
