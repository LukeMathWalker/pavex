use googletest::matcher::{self, Matcher, MatcherBase};
use pavex::cookie::ResponseCookie;
use pavex::time::{Timestamp, tz::TimeZone};

/// Check if the cookie deletes the client-side state, thus invalidating the session.
pub fn is_removal_cookie() -> RemovalCookieMatcher {
    RemovalCookieMatcher
}

#[derive(Clone, Copy, matcher::MatcherBase)]
pub struct RemovalCookieMatcher;

impl Matcher<&ResponseCookie<'static>> for RemovalCookieMatcher {
    fn matches(&self, actual: &ResponseCookie<'static>) -> matcher::MatcherResult {
        if let Some(expires) = actual.expires() {
            if let Some(expires) = expires.datetime() {
                let unix_epoch = Timestamp::UNIX_EPOCH.to_zoned(TimeZone::UTC);
                return (expires == unix_epoch).into();
            }
        }
        matcher::MatcherResult::NoMatch
    }

    fn describe(
        &self,
        matcher_result: matcher::MatcherResult,
    ) -> googletest::description::Description {
        match matcher_result {
            matcher::MatcherResult::Match => "is a removal cookie",
            matcher::MatcherResult::NoMatch => "isn't a removal cookie",
        }
        .into()
    }
}
