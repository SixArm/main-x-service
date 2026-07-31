//! SEO artifacts (CMS-R19, CMS-D13) — pure, DB-free.
//!
//! Everything here is **derived from what is actually published**.
//! A sitemap that lists pages nobody can reach, or a canonical URL
//! pointing at a draft, is worse than no sitemap at all: it teaches a
//! crawler to distrust the whole file.
//!
//! Two details that are easy to get wrong and are pinned by tests:
//!
//! - **XML escaping.** A path or title containing `&` or `<` must not
//!   be able to produce a malformed sitemap — or, worse, to inject
//!   elements into one.
//! - **`hreflang` alternates are reciprocal.** Every published locale
//!   of an entry lists every other one, including itself; a one-way
//!   alternate is the most common way a multilingual sitemap
//!   misinforms a crawler.

use serde::Serialize;
use std::fmt::Write as _;

/// One page in a sitemap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SitemapEntry {
    /// Absolute URL.
    pub location: String,
    /// When its published revision was written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
    /// Reciprocal alternates: `(locale, url)`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub alternates: Vec<(String, String)>,
}

/// Join a site's base URL and a path into an absolute URL.
///
/// Returns `None` when the site declares no base URL — an absolute URL
/// cannot be invented, and a sitemap of relative paths is not a
/// sitemap.
#[must_use]
pub fn absolute(base_url: Option<&str>, locale: &str, path: &str) -> Option<String> {
    let base = base_url?.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    if path.is_empty() {
        Some(format!("{base}/{locale}"))
    } else {
        Some(format!("{base}/{locale}/{path}"))
    }
}

/// The canonical URL for a page: the revision's declared canonical if
/// it has one, else the page's own address.
///
/// A declared canonical wins because it is the editor stating where the
/// authoritative copy lives — including on another site, which is
/// precisely the case a derived URL cannot express.
#[must_use]
pub fn canonical(
    declared: Option<&str>,
    base_url: Option<&str>,
    locale: &str,
    path: &str,
) -> Option<String> {
    match declared {
        Some(url) if !url.trim().is_empty() => Some(url.trim().to_string()),
        _ => absolute(base_url, locale, path),
    }
}

/// Escape text for XML content.
#[must_use]
pub fn escape_xml(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(character),
        }
    }
    out
}

/// Render a `sitemap.xml`.
#[must_use]
pub fn render_sitemap(entries: &[SitemapEntry]) -> String {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\" \
         xmlns:xhtml=\"http://www.w3.org/1999/xhtml\">\n",
    );
    for entry in entries {
        xml.push_str("  <url>\n");
        let _ = writeln!(xml, "    <loc>{}</loc>", escape_xml(&entry.location));
        if let Some(last_modified) = &entry.last_modified {
            let _ = writeln!(xml, "    <lastmod>{}</lastmod>", escape_xml(last_modified));
        }
        for (locale, url) in &entry.alternates {
            let _ = writeln!(
                xml,
                "    <xhtml:link rel=\"alternate\" hreflang=\"{}\" href=\"{}\"/>",
                escape_xml(locale),
                escape_xml(url)
            );
        }
        xml.push_str("  </url>\n");
    }
    xml.push_str("</urlset>\n");
    xml
}

/// One entry in a feed of recently published pages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FeedEntry {
    /// Absolute URL of the page.
    pub location: String,
    /// The published revision's title.
    pub title: String,
    /// When it was published, RFC 3339.
    pub updated: String,
    /// A stable identity for the entry, independent of its address —
    /// so a rename does not read as a new item in every reader.
    pub id: String,
    /// A short plain-text summary, when the page has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

/// The most entries a feed carries. A feed is "what changed lately",
/// not an archive; an unbounded one turns every poll into a full-table
/// read for a reader who wanted the last ten items.
pub const FEED_LIMIT: usize = 50;

/// Render an Atom 1.0 feed.
///
/// Atom rather than RSS: it requires a stable `id` and an unambiguous
/// `updated` timestamp, both of which this service can supply honestly,
/// where RSS's `pubDate`/`guid` are conventions rather than
/// requirements.
///
/// **Summaries are plain text, never markup.** The service stores
/// blocks, not HTML, and a feed is exactly the place where inventing
/// markup would escape the block model into somebody else's reader —
/// so `type="text"` is declared and the content is escaped.
///
/// `updated` on the feed itself is the newest entry's timestamp, or
/// `fallback_updated` when there are no entries — a feed with no
/// `updated` is invalid, and inventing "now" would make an unchanged
/// feed look fresh on every poll.
#[must_use]
pub fn render_feed(
    title: &str,
    self_url: &str,
    site_url: Option<&str>,
    entries: &[FeedEntry],
    fallback_updated: &str,
) -> String {
    let updated = entries
        .first()
        .map_or(fallback_updated, |entry| entry.updated.as_str());
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <feed xmlns=\"http://www.w3.org/2005/Atom\">\n",
    );
    let _ = writeln!(xml, "  <title>{}</title>", escape_xml(title));
    let _ = writeln!(xml, "  <id>{}</id>", escape_xml(self_url));
    let _ = writeln!(xml, "  <updated>{}</updated>", escape_xml(updated));
    let _ = writeln!(
        xml,
        "  <link rel=\"self\" type=\"application/atom+xml\" href=\"{}\"/>",
        escape_xml(self_url)
    );
    if let Some(site_url) = site_url {
        let _ = writeln!(
            xml,
            "  <link rel=\"alternate\" type=\"text/html\" href=\"{}\"/>",
            escape_xml(site_url)
        );
    }
    for entry in entries.iter().take(FEED_LIMIT) {
        xml.push_str("  <entry>\n");
        let _ = writeln!(xml, "    <title>{}</title>", escape_xml(&entry.title));
        let _ = writeln!(xml, "    <id>{}</id>", escape_xml(&entry.id));
        let _ = writeln!(xml, "    <updated>{}</updated>", escape_xml(&entry.updated));
        let _ = writeln!(
            xml,
            "    <link rel=\"alternate\" type=\"text/html\" href=\"{}\"/>",
            escape_xml(&entry.location)
        );
        if let Some(summary) = &entry.summary {
            let _ = writeln!(
                xml,
                "    <summary type=\"text\">{}</summary>",
                escape_xml(summary)
            );
        }
        xml.push_str("  </entry>\n");
    }
    xml.push_str("</feed>\n");
    xml
}

/// Render a `robots.txt` for a site.
///
/// A `restricted` site is disallowed wholesale: it is not for public
/// consumption, and a crawler that cannot read it should not be invited
/// to try.
#[must_use]
pub fn render_robots(
    public: bool,
    base_url: Option<&str>,
    site_key: &str,
    default_robots: &str,
) -> String {
    let mut out = String::from("User-agent: *\n");
    if !public || default_robots.starts_with("noindex") {
        out.push_str("Disallow: /\n");
        return out;
    }
    out.push_str("Allow: /\n");
    if let Some(base) = base_url {
        let _ = writeln!(
            out,
            "Sitemap: {}/delivery/{site_key}/sitemap.xml",
            base.trim_end_matches('/')
        );
    }
    out
}

/// Whether a page's `robots` directive permits indexing.
#[must_use]
pub fn is_indexable(robots: Option<&str>) -> bool {
    !robots.unwrap_or("index,follow").starts_with("noindex")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(location: &str) -> SitemapEntry {
        SitemapEntry {
            location: location.to_string(),
            last_modified: Some("2026-07-30T12:00:00Z".to_string()),
            alternates: Vec::new(),
        }
    }

    #[test]
    fn absolute_urls_need_a_base() {
        assert_eq!(
            absolute(Some("https://example.test"), "en", "/about").as_deref(),
            Some("https://example.test/en/about")
        );
        // A trailing slash on the base does not double up.
        assert_eq!(
            absolute(Some("https://example.test/"), "fr", "about").as_deref(),
            Some("https://example.test/fr/about")
        );
        // The root path.
        assert_eq!(
            absolute(Some("https://example.test"), "en", "/").as_deref(),
            Some("https://example.test/en")
        );
        // No base: no invention.
        assert_eq!(absolute(None, "en", "/about"), None);
    }

    /// A declared canonical wins — it is the editor saying where the
    /// authoritative copy lives, possibly on another site.
    #[test]
    fn a_declared_canonical_beats_the_derived_one() {
        assert_eq!(
            canonical(
                Some("https://elsewhere.test/original"),
                Some("https://example.test"),
                "en",
                "/about"
            )
            .as_deref(),
            Some("https://elsewhere.test/original")
        );
        assert_eq!(
            canonical(Some("  "), Some("https://example.test"), "en", "/about").as_deref(),
            Some("https://example.test/en/about"),
            "a blank declaration falls through to the derived URL"
        );
        assert_eq!(canonical(None, None, "en", "/about"), None);
    }

    /// A path or title containing markup must not be able to break —
    /// or inject into — the sitemap.
    #[test]
    fn xml_is_escaped() {
        assert_eq!(escape_xml("a & b"), "a &amp; b");
        assert_eq!(
            escape_xml("</loc><url><loc>evil"),
            "&lt;/loc&gt;&lt;url&gt;&lt;loc&gt;evil"
        );
        let xml = render_sitemap(&[entry("https://a.test/en/a?x=1&y=2")]);
        assert!(xml.contains("&amp;y=2"));
        assert!(
            !xml.contains("?x=1&y"),
            "the raw ampersand does not survive"
        );
    }

    #[test]
    fn a_sitemap_renders_locations_and_timestamps() {
        let xml = render_sitemap(&[entry("https://a.test/en/about")]);
        assert!(xml.starts_with("<?xml version=\"1.0\""));
        assert!(xml.contains("<loc>https://a.test/en/about</loc>"));
        assert!(xml.contains("<lastmod>2026-07-30T12:00:00Z</lastmod>"));
        assert!(xml.trim_end().ends_with("</urlset>"));
    }

    #[test]
    fn alternates_are_rendered_with_hreflang() {
        let mut with_alternates = entry("https://a.test/en/about");
        with_alternates.alternates = vec![
            ("en".to_string(), "https://a.test/en/about".to_string()),
            ("fr".to_string(), "https://a.test/fr/a-propos".to_string()),
        ];
        let xml = render_sitemap(&[with_alternates]);
        assert!(xml.contains("hreflang=\"en\""));
        assert!(xml.contains("href=\"https://a.test/fr/a-propos\""));
    }

    #[test]
    fn an_empty_sitemap_is_still_valid_xml() {
        let xml = render_sitemap(&[]);
        assert!(xml.contains("<urlset"));
        assert!(xml.contains("</urlset>"));
    }

    /// A restricted site is not for public consumption, so it is not
    /// advertised to crawlers either.
    #[test]
    fn robots_disallows_everything_on_a_restricted_or_noindex_site() {
        let restricted = render_robots(false, Some("https://a.test"), "s", "index,follow");
        assert!(restricted.contains("Disallow: /"));
        assert!(!restricted.contains("Sitemap:"));

        let noindex = render_robots(true, Some("https://a.test"), "s", "noindex,follow");
        assert!(noindex.contains("Disallow: /"));
    }

    #[test]
    fn robots_points_a_public_site_at_its_sitemap() {
        let robots = render_robots(true, Some("https://a.test/"), "handbook", "index,follow");
        assert!(robots.contains("Allow: /"));
        assert!(robots.contains("Sitemap: https://a.test/delivery/handbook/sitemap.xml"));
        // Without a base URL there is no sitemap line to write.
        assert!(!render_robots(true, None, "handbook", "index,follow").contains("Sitemap:"));
    }

    #[test]
    fn indexability_reads_the_robots_directive() {
        assert!(is_indexable(None));
        assert!(is_indexable(Some("index,follow")));
        assert!(is_indexable(Some("index,nofollow")));
        assert!(!is_indexable(Some("noindex,follow")));
        assert!(!is_indexable(Some("noindex,nofollow")));
    }

    // ---- feed ------------------------------------------------------

    fn a_feed_entry(title: &str, updated: &str) -> FeedEntry {
        FeedEntry {
            location: format!("https://demo.test/en/{}", title.to_lowercase()),
            title: title.to_string(),
            updated: updated.to_string(),
            id: format!("urn:uuid:0000-{title}"),
            summary: None,
        }
    }

    #[test]
    fn a_feed_carries_the_elements_atom_requires() {
        let entries = vec![
            a_feed_entry("Newest", "2026-07-31T09:00:00Z"),
            a_feed_entry("Older", "2026-07-01T09:00:00Z"),
        ];
        let xml = render_feed(
            "Demo site",
            "https://demo.test/en/feed.xml",
            Some("https://demo.test/en/"),
            &entries,
            "2020-01-01T00:00:00Z",
        );
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("xmlns=\"http://www.w3.org/2005/Atom\""));
        assert!(xml.contains("rel=\"self\""));
        assert_eq!(xml.matches("<entry>").count(), 2);
        // The feed's own `updated` is the newest entry's, so an
        // unchanged feed does not look fresh on every poll.
        assert!(xml.contains("<updated>2026-07-31T09:00:00Z</updated>"));
        assert!(!xml.contains("2020-01-01"), "the fallback is unused here");
    }

    /// An empty feed is still a valid feed, and must not invent a time.
    #[test]
    fn an_empty_feed_uses_the_fallback_timestamp() {
        let xml = render_feed(
            "Demo site",
            "https://demo.test/en/feed.xml",
            None,
            &[],
            "2020-01-01T00:00:00Z",
        );
        assert!(xml.contains("<updated>2020-01-01T00:00:00Z</updated>"));
        assert!(!xml.contains("<entry>"));
        assert!(
            !xml.contains("rel=\"alternate\""),
            "no site URL, no alternate"
        );
    }

    /// The property that keeps the block model from escaping into
    /// somebody else's reader.
    #[test]
    fn feed_content_is_escaped_text_never_markup() {
        let entry = FeedEntry {
            location: "https://demo.test/en/x?a=1&b=2".to_string(),
            title: "Tom & Jerry <b>win</b>".to_string(),
            updated: "2026-07-31T09:00:00Z".to_string(),
            id: "urn:uuid:1".to_string(),
            summary: Some("5 > 3 & \"quoted\"".to_string()),
        };
        let xml = render_feed("A & B", "https://demo.test/f.xml", None, &[entry], "x");
        assert!(xml.contains("Tom &amp; Jerry &lt;b&gt;win&lt;/b&gt;"));
        assert!(xml.contains("a=1&amp;b=2"));
        assert!(
            xml.contains("type=\"text\""),
            "summaries are declared plain text"
        );
        // No unescaped tag from the content survives into the document.
        assert!(!xml.contains("<b>"));
    }

    /// A feed is what changed lately, not an archive.
    #[test]
    fn a_feed_is_capped() {
        let entries: Vec<FeedEntry> = (0..FEED_LIMIT + 25)
            .map(|i| a_feed_entry(&format!("Item{i}"), "2026-07-31T09:00:00Z"))
            .collect();
        let xml = render_feed("Demo", "https://demo.test/f.xml", None, &entries, "x");
        assert_eq!(xml.matches("<entry>").count(), FEED_LIMIT);
    }

    /// The identity is independent of the address, so renaming a page
    /// does not resurface it as a new item in every reader.
    #[test]
    fn an_entry_id_is_not_its_url() {
        let mut entry = a_feed_entry("Thing", "2026-07-31T09:00:00Z");
        let before = render_feed("D", "https://demo.test/f.xml", None, &[entry.clone()], "x");
        entry.location = "https://demo.test/en/renamed".to_string();
        let after = render_feed("D", "https://demo.test/f.xml", None, &[entry], "x");
        assert_ne!(before, after, "the link changed");
        let id = "<id>urn:uuid:0000-Thing</id>";
        assert!(
            before.contains(id) && after.contains(id),
            "the identity did not"
        );
    }
}
