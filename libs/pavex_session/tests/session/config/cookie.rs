//! Verify that all cookie settings behave as expected.
use googletest::{
    expect_that,
    matchers::anything,
    prelude::{eq, none, some},
};
use pavex::cookie::{RequestCookie, RequestCookies, SameSite};
use pavex_session::{
    config::{SessionCookieConfig, SessionCookieKind},
    IncomingSession, Session, SessionConfig,
};

use crate::fixtures::{store, SessionFixture};

#[tokio::test]
#[googletest::test]
async fn cookie_attributes_can_be_changed() {
    let (store, mut config) = (store(), SessionConfig::default());
    config.cookie.name = "my-custom-cookie-name".into();
    config.cookie.domain = Some("my-domain.com".into());
    config.cookie.path = Some("/custom-path".into());
    config.cookie.secure = false;
    config.cookie.http_only = false;
    config.cookie.same_site = Some(SameSite::Strict);
    config.cookie.kind = SessionCookieKind::Session;

    let fixture = SessionFixture::default();
    let incoming = fixture.setup(&store).await;
    let mut session = Session::new(&store, &config, Some(incoming));

    let cookie = session.finalize().await.unwrap().unwrap();
    expect_that!(cookie.name(), eq(config.cookie.name));
    expect_that!(cookie.domain(), eq(config.cookie.domain.as_deref()));
    expect_that!(cookie.path(), eq(config.cookie.path.as_deref()));
    expect_that!(cookie.secure(), none());
    expect_that!(cookie.http_only(), none());
    expect_that!(cookie.same_site(), eq(config.cookie.same_site));
    expect_that!(cookie.max_age(), none());
    expect_that!(cookie.expires(), none());
    expect_that!(cookie.expires_datetime(), none());
}

#[tokio::test]
#[googletest::test]
async fn default_cookie_settings() {
    let (store, config) = (store(), SessionConfig::default());

    let fixture = SessionFixture::default();
    let incoming = fixture.setup(&store).await;
    let mut session = Session::new(&store, &config, Some(incoming));

    let cookie = session.finalize().await.unwrap().unwrap();
    expect_that!(cookie.name(), eq("id"));
    expect_that!(cookie.path(), some(eq("/")));
    expect_that!(cookie.domain(), none());
    expect_that!(cookie.http_only(), some(eq(true)));
    expect_that!(cookie.secure(), some(eq(true)));
    expect_that!(cookie.same_site(), some(eq(SameSite::Lax)));
    expect_that!(cookie.max_age(), some(anything()));
}

#[googletest::test]
fn serialize_same_site() {
    use pavex::cookie::SameSite;
    use serde_json;

    let same_site = Some(SameSite::Strict);
    let serialized = serde_json::to_string(&same_site).unwrap();
    expect_that!(serialized, eq("\"Strict\""));

    let same_site = Some(SameSite::Lax);
    let serialized = serde_json::to_string(&same_site).unwrap();
    expect_that!(serialized, eq("\"Lax\""));

    let same_site = Some(SameSite::None);
    let serialized = serde_json::to_string(&same_site).unwrap();
    expect_that!(serialized, eq("\"None\""));

    let same_site: Option<SameSite> = None;
    let serialized = serde_json::to_string(&same_site).unwrap();
    expect_that!(serialized, eq("null"));
}

#[googletest::test]
fn deserialize_same_site() {
    use pavex::cookie::SameSite;
    use serde_json;

    let json = "\"Strict\"";
    let deserialized: Option<SameSite> = serde_json::from_str(json).unwrap();
    expect_that!(deserialized, eq(Some(SameSite::Strict)));

    let json = "\"Lax\"";
    let deserialized: Option<SameSite> = serde_json::from_str(json).unwrap();
    expect_that!(deserialized, eq(Some(SameSite::Lax)));

    let json = "\"None\"";
    let deserialized: Option<SameSite> = serde_json::from_str(json).unwrap();
    expect_that!(deserialized, eq(Some(SameSite::None)));

    let json = "null";
    let deserialized: Option<SameSite> = serde_json::from_str(json).unwrap();
    expect_that!(deserialized, eq(None));
}

#[tokio::test]
#[googletest::test]
async fn incoming_looks_for_the_right_cookie_name() {
    // Create a valid session cookie.
    let value = {
        let (store, config) = (store(), SessionConfig::default());
        let fixture = SessionFixture::default();
        let incoming = fixture.setup(&store).await;
        let mut session = Session::new(&store, &config, Some(incoming));
        let cookie = session.finalize().await.unwrap().unwrap();
        cookie.value().to_owned()
    };

    // The cookie name matches, so it's `Some`
    let mut cookie_config = SessionCookieConfig::default();
    cookie_config.name = "my-custom-cookie-name".into();
    let mut request_cookies = RequestCookies::new();
    request_cookies.append(RequestCookie::new(&cookie_config.name, value));
    assert!(IncomingSession::extract(&request_cookies, &cookie_config).is_some());

    // The cookie name doesn't match, now it's `None`
    let mut cookie_config = SessionCookieConfig::default();
    cookie_config.name = "another-name".into();
    assert!(IncomingSession::extract(&request_cookies, &cookie_config).is_none());

    // The cookie name matches, but the value is not a valid state, so it's again `None`
    let cookie_config = SessionCookieConfig::default();
    let mut request_cookies = RequestCookies::new();
    request_cookies.append(RequestCookie::new(&cookie_config.name, "gibberish"));
    assert!(IncomingSession::extract(&request_cookies, &cookie_config).is_none());
}
