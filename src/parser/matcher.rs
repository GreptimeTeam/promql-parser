type MatchType = i32;

// Matcher models the matching of a label.
pub struct Matcher {
    mtype: MatchType,
    name: String,
    value: String,
    // FIXME:
    // re *FastRegexMatcher
}
