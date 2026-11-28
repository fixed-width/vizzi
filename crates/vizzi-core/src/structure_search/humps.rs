use icu_properties::{
    CodePointMapData,
    props::{GeneralCategory, GeneralCategoryGroup},
};
use std::borrow::Cow;
use unicase::UniCase;

pub(super) struct Humps {
    needle: Vec<char>,
    reachable: Vec<bool>,
    ascii: bool,
    case_sensitive: bool,
}

impl Humps {
    pub(super) fn new(input: &str, case_sensitive: bool) -> Option<Self> {
        let mut chars = input.chars();
        if is_mark(chars.next()?) || chars.as_str().is_empty() || !input.chars().all(is_word_char) {
            return None;
        }
        let folded = if case_sensitive {
            Cow::Borrowed(input)
        } else {
            Cow::Owned(UniCase::new(input).to_folded_case())
        };
        let needle: Vec<_> = folded.chars().collect();
        Some(Self {
            reachable: vec![false; needle.len()],
            needle,
            ascii: folded.is_ascii(),
            case_sensitive,
        })
    }

    pub(super) fn matches(&mut self, name: &str) -> bool {
        if self.ascii && name.is_ascii() && !self.ascii_subsequence(name) {
            return false;
        }
        self.reachable.fill(false);
        self.reachable[0] = true;
        let mut furthest = 0;
        for part in name.split(|ch| !is_word_char(ch)) {
            let mut chars = part
                .char_indices()
                .filter(|&(_, ch)| !is_mark(ch))
                .peekable();
            let Some((mut start, mut previous)) = chars.next() else {
                continue;
            };
            while let Some((end, current)) = chars.next() {
                let next = chars.peek().map(|&(_, ch)| ch);
                if starts_word(previous, current, next) {
                    if self.match_word(&part[start..end], &mut furthest) {
                        return true;
                    }
                    start = end;
                }
                previous = current;
            }
            if self.match_word(&part[start..], &mut furthest) {
                return true;
            }
        }
        false
    }

    fn ascii_subsequence(&self, name: &str) -> bool {
        let mut bytes = name.bytes();
        self.needle
            .iter()
            .all(|&expected| bytes.any(|actual| self.fold_ascii(char::from(actual)) == expected))
    }

    fn match_word(&mut self, word: &str, furthest: &mut usize) -> bool {
        let folded =
            (!self.case_sensitive && !word.is_ascii()).then(|| UniCase::new(word).to_folded_case());
        let word = folded.as_deref().unwrap_or(word);
        // Descending offsets keep prefixes reached in this word from using it again.
        for start in (0..=*furthest).rev() {
            if !self.reachable[start] {
                continue;
            }
            let length = word
                .chars()
                .zip(&self.needle[start..])
                .take_while(|&(actual, &expected)| self.fold_ascii(actual) == expected)
                .count();
            let end = start + length;
            if end == self.needle.len() {
                return true;
            }
            self.reachable[start + 1..end + 1].fill(true);
            *furthest = (*furthest).max(end);
        }
        false
    }

    fn fold_ascii(&self, ch: char) -> char {
        if self.case_sensitive {
            ch
        } else {
            ch.to_ascii_lowercase()
        }
    }
}

fn starts_word(previous: char, current: char, next: Option<char>) -> bool {
    previous.is_numeric() != current.is_numeric()
        || current.is_uppercase()
            && (previous.is_lowercase()
                || previous.is_uppercase() && next.is_some_and(char::is_lowercase))
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || is_mark(ch)
}

fn is_mark(ch: char) -> bool {
    !ch.is_ascii()
        && GeneralCategoryGroup::Mark.contains(CodePointMapData::<GeneralCategory>::new().get(ch))
}
