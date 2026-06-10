//! Nickname equivalence tables for given-name matching.
//!
//! Person records routinely carry the same person under several given-name
//! variants — `Michael` vs `Mike`, `Elizabeth` vs `Liz`, `Robert` vs `Bob`.
//! No string-similarity metric (Jaro-Winkler, Levenshtein, Soundex) closes
//! that gap on its own because the variants don't share enough characters
//! or phonemes. The fix is a small lookup table of known equivalence
//! classes, applied as a **post-similarity boost** so the matcher can lift
//! the given-name score when both records carry forms that the table
//! knows about.
//!
//! ## API
//!
//! Tables are constructed via [`NicknameTable::empty`] or
//! [`NicknameTable::english`] (a built-in default for common English
//! nicknames). Additional classes are added with
//! [`NicknameTable::with_class`].
//!
//! Two names are *equivalent* under a table iff, after
//! [`crate::Normalizer::normalize_name`] is applied, both end up in the
//! same equivalence class. Identical normalised strings are trivially
//! equivalent (the table does not need to list them explicitly).
//!
//! ## Integration with the matcher
//!
//! [`crate::MatchConfig::nickname_table`] is empty by default — nicknames
//! are an opt-in feature so existing behaviour is preserved. When a
//! non-empty table is configured, the matcher's name scoring computes the
//! configured similarity algorithm as usual and then **lifts the score to
//! `0.9` if-and-only-if the table considers the pair equivalent**. The
//! boost never lowers a score.
//!
//! ## Scope and limitations
//!
//! - English-language nicknames only. Localised tables are tracked in
//!   `spec.md` §21 medium-term work and can be slotted in by constructing
//!   a fresh [`NicknameTable`] at the call site.
//! - One-way ambiguity is intentional: `Sandy` can be a nickname for
//!   `Alexandra` or `Sandra`. Both are listed so that either canonical
//!   form matches `Sandy`; lookups return `true` when the two normalised
//!   inputs share *any* class.
//! - Family names are out of scope. The matcher applies the table to
//!   both given and family names because `score_name` is shared, but the
//!   default English table contains no family-name entries.
//!
//! # Examples
//!
//! ```
//! use person_matcher::NicknameTable;
//!
//! let table = NicknameTable::english();
//! assert!(table.are_equivalent("Mike", "Michael"));
//! assert!(table.are_equivalent("Liz", "Elizabeth"));
//! assert!(table.are_equivalent("Bob", "Robert"));
//! assert!(!table.are_equivalent("Mike", "Robert"));
//!
//! // Add a custom class on top:
//! let table = NicknameTable::english().with_class(["Reginald", "Reggie"]);
//! assert!(table.are_equivalent("Reggie", "Reginald"));
//! ```

use crate::normalizer::Normalizer;
use serde::{Deserialize, Serialize};

/// Equivalence-class lookup table for given-name nicknames.
///
/// Each class is a `Vec<String>` of normalised forms that the table
/// considers interchangeable. Two inputs are equivalent under the table
/// iff their normalised forms appear in the same class — or are
/// byte-identical after normalisation.
///
/// The type is `Clone + Debug + PartialEq + Eq` so it composes into
/// [`crate::MatchConfig`] without surprises. Construction is cheap and
/// allocates once per class; lookup is `O(classes × entries)` and is
/// dominated by the table's size, not the input string length.
///
/// # Example
///
/// ```
/// use person_matcher::NicknameTable;
///
/// let t = NicknameTable::empty()
///     .with_class(["Michael", "Mike", "Mickey"])
///     .with_class(["Elizabeth", "Liz", "Beth"]);
///
/// assert!(t.are_equivalent("Mike", "Michael"));
/// assert!(t.are_equivalent("liz", "BETH"));
/// assert!(!t.are_equivalent("Michael", "Liz"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NicknameTable {
    /// Equivalence classes; all strings within one inner `Vec` are treated
    /// as interchangeable, and strings in different classes never match.
    classes: Vec<Vec<String>>,
}

impl NicknameTable {
    /// Construct an empty table that considers every pair of distinct
    /// strings non-equivalent (identical strings remain trivially equal).
    ///
    /// ```
    /// use person_matcher::NicknameTable;
    /// let t = NicknameTable::empty();
    /// assert!(!t.are_equivalent("Mike", "Michael"));
    /// assert!(t.are_equivalent("Mike", "Mike"));
    /// ```
    #[must_use]
    pub fn empty() -> Self {
        Self {
            classes: Vec::new(),
        }
    }

    /// Append an equivalence class to the table.
    ///
    /// Each input string is normalised via
    /// [`crate::Normalizer::normalize_name`] before insertion so the
    /// table is closed under the same normalisation pipeline the matcher
    /// uses at lookup time. Duplicate or empty entries are silently
    /// dropped, and a class with fewer than two distinct normalised
    /// entries is dropped entirely (it would never make a pair
    /// equivalent).
    ///
    /// ```
    /// use person_matcher::NicknameTable;
    /// let t = NicknameTable::empty().with_class(["Robert", "Bob", "Rob"]);
    /// assert!(t.are_equivalent("BOB", "robert"));
    /// ```
    pub fn with_class<I, S>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut entries: Vec<String> = Vec::new();
        for name in names {
            let normalised = Normalizer::normalize_name(name.as_ref());
            if !normalised.is_empty() && !entries.contains(&normalised) {
                entries.push(normalised);
            }
        }
        if entries.len() >= 2 {
            self.classes.push(entries);
        }
        self
    }

    /// Return `true` iff `a` and `b`, after name normalisation, are
    /// considered the same person by this table.
    ///
    /// Identical normalised strings are trivially equivalent. Otherwise
    /// both inputs must appear in the same equivalence class.
    ///
    /// ```
    /// use person_matcher::NicknameTable;
    /// let t = NicknameTable::english();
    /// assert!(t.are_equivalent("Mike",  "Michael"));
    /// assert!(t.are_equivalent("mike",  "MICHAEL"));   // case-insensitive
    /// assert!(!t.are_equivalent("Mike", "Robert"));
    /// assert!(t.are_equivalent("",       ""));         // trivially equal
    /// ```
    #[must_use]
    pub fn are_equivalent(&self, a: &str, b: &str) -> bool {
        let na = Normalizer::normalize_name(a);
        let nb = Normalizer::normalize_name(b);
        if na == nb {
            return true;
        }
        self.classes
            .iter()
            .any(|cls| cls.iter().any(|n| n == &na) && cls.iter().any(|n| n == &nb))
    }

    /// `true` iff the table contains no equivalence classes — equivalent
    /// to comparing with [`NicknameTable::empty`] but cheaper to test.
    ///
    /// ```
    /// use person_matcher::NicknameTable;
    /// assert!(NicknameTable::empty().is_empty());
    /// assert!(!NicknameTable::english().is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.classes.is_empty()
    }

    /// Number of equivalence classes registered with this table.
    ///
    /// ```
    /// use person_matcher::NicknameTable;
    /// assert_eq!(NicknameTable::empty().len(), 0);
    /// let t = NicknameTable::empty().with_class(["A", "B"]);
    /// assert_eq!(t.len(), 1);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        self.classes.len()
    }

    /// A built-in table covering the most common English-language
    /// nicknames encountered in healthcare data: `Michael`/`Mike`,
    /// `Robert`/`Bob`, `Elizabeth`/`Liz`, and similar.
    ///
    /// The exact contents are not part of the public contract — entries
    /// may be added in minor releases. Callers that need a stable
    /// dictionary SHOULD construct their own via
    /// [`NicknameTable::with_class`].
    ///
    /// ```
    /// use person_matcher::NicknameTable;
    /// let t = NicknameTable::english();
    /// assert!(t.are_equivalent("Bill",      "William"));
    /// assert!(t.are_equivalent("Liz",       "Elizabeth"));
    /// assert!(t.are_equivalent("Steve",     "Steven"));
    /// assert!(t.are_equivalent("Steve",     "Stephen"));
    /// ```
    #[must_use]
    pub fn english() -> Self {
        let pairs: &[&[&str]] = &[
            &["michael", "mike", "mick", "mickey"],
            &["robert", "bob", "rob", "robbie", "bobby"],
            &["william", "will", "bill", "billy", "willy"],
            &["james", "jim", "jimmy", "jamie"],
            &["richard", "rick", "dick", "rich", "richie"],
            &["thomas", "tom", "tommy"],
            &[
                "elizabeth",
                "liz",
                "beth",
                "betty",
                "eliza",
                "lizzy",
                "betsy",
            ],
            &[
                "katherine",
                "kate",
                "kathy",
                "katy",
                "kat",
                "cathy",
                "katie",
            ],
            &[
                "catherine",
                "kate",
                "kathy",
                "katy",
                "kat",
                "cathy",
                "katie",
            ],
            &["margaret", "maggie", "meg", "peggy", "marge"],
            &["jennifer", "jen", "jenny", "jenn"],
            &["patricia", "pat", "patty", "tricia", "trish"],
            &["susan", "sue", "suzie", "susie"],
            &["barbara", "barb", "babs"],
            &["anthony", "tony"],
            &["christopher", "chris", "kris"],
            &["charles", "charlie", "chuck", "chas"],
            &["daniel", "dan", "danny"],
            &["david", "dave", "davy"],
            &["edward", "ed", "eddie", "ted", "ned"],
            &["joseph", "joe", "joey"],
            &["kenneth", "ken", "kenny"],
            &["nicholas", "nick", "nico"],
            &["peter", "pete"],
            &["samuel", "sam", "sammy"],
            &["stephen", "steve", "stevie"],
            &["steven", "steve", "stevie"],
            &["timothy", "tim", "timmy"],
            &["alexander", "alex", "xander"],
            &["alexandra", "alex", "alexa", "sandy"],
            &["sandra", "sandy"],
            &["benjamin", "ben", "benny"],
            &["rebecca", "becca", "becky"],
            &["sarah", "sara", "sally"],
            &["victoria", "vicky", "vic", "tori"],
            &["matthew", "matt", "matty"],
            &["jonathan", "jon", "jonny", "jonathon"],
            &["frederick", "fred", "freddy", "freddie"],
            &["lawrence", "larry"],
            &["henry", "hank", "harry"],
            &["ronald", "ron", "ronnie"],
            &["donald", "don", "donnie"],
            &["andrew", "andy", "drew"],
            &["abigail", "abby", "gail"],
            &["amanda", "mandy"],
            &["isabella", "izzy", "bella"],
            &["isabel", "izzy", "bella"],
            &["olivia", "liv", "livy"],
            &["nicole", "nikki"],
            &["samantha", "sam", "sammy"],
            &["pamela", "pam"],
            &["deborah", "deb", "debbie"],
            &["kimberly", "kim"],
            &["jessica", "jess", "jessie"],
            &["stephanie", "steph"],
            &["madeline", "maddy", "maddie"],
        ];
        let mut table = Self::empty();
        for class in pairs {
            table = table.with_class(*class);
        }
        table
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_table_treats_distinct_strings_as_inequivalent() {
        let t = NicknameTable::empty();
        assert!(!t.are_equivalent("Mike", "Michael"));
        assert!(!t.are_equivalent("Liz", "Elizabeth"));
    }

    #[test]
    fn identical_normalised_strings_are_trivially_equivalent_even_when_empty() {
        let t = NicknameTable::empty();
        assert!(t.are_equivalent("Mike", "mike"));
        assert!(t.are_equivalent("MICHAEL", "michael"));
        assert!(t.are_equivalent("", ""));
    }

    #[test]
    fn with_class_normalises_entries_at_insertion() {
        let t = NicknameTable::empty().with_class(["Robert", "Bob", "Rob"]);
        assert!(t.are_equivalent("BOB", "robert"));
        assert!(t.are_equivalent("Rob", "Robert"));
    }

    #[test]
    fn with_class_dedupes_after_normalisation() {
        let t = NicknameTable::empty().with_class(["mike", "MIKE", "Mike"]);
        // All three normalise to "mike"; class collapses to a single
        // entry and is therefore dropped (no pair makes the class useful).
        assert_eq!(t.len(), 0);
    }

    #[test]
    fn with_class_drops_classes_with_fewer_than_two_distinct_entries() {
        let t = NicknameTable::empty().with_class(["Mike"]);
        assert!(t.is_empty());
    }

    #[test]
    fn with_class_drops_empty_strings_silently() {
        let t = NicknameTable::empty().with_class(["", "Mike", ""]);
        // After empties are dropped only "mike" remains → no useful class.
        assert!(t.is_empty());
    }

    #[test]
    fn english_table_covers_acceptance_criterion() {
        let t = NicknameTable::english();
        for (a, b) in [
            ("Mike", "Michael"),
            ("Liz", "Elizabeth"),
            ("Bob", "Robert"),
            ("Bill", "William"),
            ("Dick", "Richard"),
        ] {
            assert!(t.are_equivalent(a, b), "{a:?} ↮ {b:?} in english()");
        }
    }

    #[test]
    fn english_table_treats_unrelated_names_as_inequivalent() {
        let t = NicknameTable::english();
        assert!(!t.are_equivalent("Mike", "Robert"));
        assert!(!t.are_equivalent("Liz", "Tom"));
    }

    #[test]
    fn english_table_handles_shared_nicknames_across_classes() {
        let t = NicknameTable::english();
        // "Sandy" is a recognised nickname for both Alexandra and Sandra;
        // matching against either canonical succeeds.
        assert!(t.are_equivalent("Sandy", "Alexandra"));
        assert!(t.are_equivalent("Sandy", "Sandra"));
        // Steve appears in both Stephen and Steven classes for similar
        // reasons.
        assert!(t.are_equivalent("Steve", "Stephen"));
        assert!(t.are_equivalent("Steve", "Steven"));
    }

    #[test]
    fn with_class_composes_on_top_of_english() {
        let t = NicknameTable::english().with_class(["Reginald", "Reggie"]);
        assert!(t.are_equivalent("Reggie", "Reginald"));
        // Original entries still work.
        assert!(t.are_equivalent("Mike", "Michael"));
    }

    #[test]
    fn lookup_is_case_and_punctuation_insensitive() {
        let t = NicknameTable::english();
        assert!(t.are_equivalent("MIKE", "michael"));
        assert!(t.are_equivalent("  Mike  ", "Michael"));
    }

    #[test]
    fn default_is_empty() {
        let t = NicknameTable::default();
        assert!(t.is_empty());
    }
}
