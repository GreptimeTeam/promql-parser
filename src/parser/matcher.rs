type MatchType = i32;

// Matcher models the matching of a label.
// FIXME: this is just the skeleton, details need to be done
#[derive(Debug)]
pub struct Matcher {
    mtype: MatchType,
    name: String,
    value: String,
    // FIXME:
    // re *FastRegexMatcher
}
