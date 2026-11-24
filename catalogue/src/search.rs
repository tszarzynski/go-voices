//! Finding patches by what you type.

use alloc::{string::String, vec::Vec};

use crate::Patch;

/// What's typed in the search field, prepared once to rank every patch.
#[derive(Clone, Debug)]
pub struct Query {
    /// Trimmed and lowercase.
    text: String,
    words: Vec<String>,
}

impl Query {
    /// Prepares `text` for matching.
    #[must_use]
    pub fn new(text: &str) -> Self {
        let text = text.trim().to_ascii_lowercase();
        let words = text.split_whitespace().map(String::from).collect();
        Self { text, words }
    }

    /// How well `patch` matches, or `None` if it doesn't. Lower is better:
    /// 0 when the name starts with the query, 1 when every word typed
    /// starts a word of the name, 2 for any other match.
    ///
    /// Every word of the query must appear, ignoring case, in the patch's
    /// name or its category's name or code: `warm pad` finds `Warm Pad`, and
    /// `Warm Strings` in the soft pad category. An empty query matches
    /// everything.
    #[must_use]
    pub fn rank(&self, patch: &Patch) -> Option<u8> {
        if self.words.is_empty() {
            return Some(0);
        }
        let name = patch.name.to_ascii_lowercase();
        let mut category = [patch.category.name(), " ", patch.category.code()].concat();
        category.make_ascii_lowercase();
        if !self
            .words
            .iter()
            .all(|word| name.contains(word.as_str()) || category.contains(word.as_str()))
        {
            return None;
        }
        if name.starts_with(self.text.as_str()) {
            return Some(0);
        }
        let name_words = name
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|w| !w.is_empty());
        let starts_words = self
            .words
            .iter()
            .all(|word| name_words.clone().any(|w| w.starts_with(word.as_str())));
        Some(if starts_words { 1 } else { 2 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patch(line: &str) -> Patch {
        line.parse().unwrap()
    }

    #[test]
    fn every_word_must_match_the_name_or_category() {
        let pad = patch("Warm Strings\tSPD\t87\t71\t1");
        assert_eq!(Query::new("warm pad").rank(&pad), Some(2));
        assert_eq!(Query::new("WARM spd").rank(&pad), Some(2));
        assert_eq!(Query::new("warm brass").rank(&pad), None);
    }

    #[test]
    fn ranks_name_prefixes_first() {
        let warm_pad = patch("Warm Pad\tSPD\t87\t71\t2");
        let soft_warm = patch("Soft Warm Pad\tSPD\t87\t71\t3");
        let swarmy = patch("Swarmy Pad\tSPD\t87\t71\t4");
        assert_eq!(Query::new("warm pad").rank(&warm_pad), Some(0));
        assert_eq!(Query::new("warm pad").rank(&soft_warm), Some(1));
        assert_eq!(Query::new("warm pad").rank(&swarmy), Some(2));
    }

    #[test]
    fn an_empty_query_matches_everything() {
        assert_eq!(Query::new("  ").rank(&patch("A\tPNO\t87\t64\t1")), Some(0));
    }
}
