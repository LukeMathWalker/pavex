use googletest::matcher::{self, Matcher, MatcherBase};
use pavex::cookie::ResponseCookie;
use time::OffsetDateTime;

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
                let date = time::Date::from_calendar_date(1970, time::Month::January, 1).unwrap();
                let time = time::Time::from_hms(0, 0, 0).unwrap();
                let offset = time::UtcOffset::from_whole_seconds(0).unwrap();
                let unix_start_datetime = OffsetDateTime::new_in_offset(date, time, offset);
                return (expires == unix_start_datetime).into();
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
