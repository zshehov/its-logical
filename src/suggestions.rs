use std::{cmp::Reverse, collections::BTreeSet, vec::IntoIter};

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

pub(crate) trait Suggestion {
    fn new(value: &str) -> Self;
}

// usually just a String is the Suggestion
impl Suggestion for String {
    fn new(value: &str) -> Self {
        value.to_string()
    }
}

pub(crate) trait Suggestions<T: Suggestion> {
    type All: Iterator<Item = T>;

    fn filter(&self, with: &str) -> Self::All;
}

pub(crate) struct FuzzySuggestions {
    fuzzy_matcher: SkimMatcherV2,
    relevant: Vec<String>,
}

impl FuzzySuggestions {
    pub(crate) fn new(relevant: impl Iterator<Item = String>) -> Self {
        let relevant_set: BTreeSet<String> = BTreeSet::from_iter(relevant);

        Self {
            fuzzy_matcher: SkimMatcherV2::default(),
            relevant: Vec::from_iter::<BTreeSet<String>>(relevant_set),
        }
    }
}

// Prouduces fuzzy suggestions sorted by fuzzy score
impl<T: Suggestion> Suggestions<T> for FuzzySuggestions {
    type All = IntoIter<T>;

    fn filter(&self, with: &str) -> IntoIter<T> {
        let mut filtered: Vec<(&String, i64)> = self
            .relevant
            .iter()
            .filter_map(|x| {
                if let Some(score) = self.fuzzy_matcher.fuzzy_match(x, with) {
                    return Some((x, score));
                }
                None
            })
            .collect();

        filtered.sort_unstable_by_key(|(_, x)| Reverse(*x));
        filtered
            .into_iter()
            .map(|(x, _)| T::new(x))
            .collect::<Vec<T>>()
            .into_iter()
    }
}
