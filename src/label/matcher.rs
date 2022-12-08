// Possible MatchTypes.
#[derive(Debug)]
pub enum MatchType {
    MatchEqual,
    MatchNotEqual,
    MatchRegexp,
    MatchNotRegexp,
}

// Matcher models the matching of a label.
#[derive(Debug)]
pub struct Matcher {
    typ: MatchType,
    name: String,
    value: String,
    // FIXME: Regex Matcher
    // re *FastRegexMatcher
}

impl Matcher {
    pub fn new(t: MatchType, n: &str, v: &str) -> Self {
        Self {
            typ: t,
            name: n.into(),
            value: n.into(),
        }
    }

    // Matches returns whether the matcher matches the given string value.
    pub fn matches(&self, s: &str) -> bool {
        match self.typ {
            MatchType::MatchEqual => self.value.eq(s),
            MatchType::MatchNotEqual => self.value.ne(s),
            MatchType::MatchRegexp => todo!(),
            MatchType::MatchNotRegexp => todo!(),
        }
    }
}
