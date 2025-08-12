use crate::fixtures::store;
use insta::assert_snapshot;
use pavex::{
    Response,
    cookie::{
        Key, ProcessorConfig, ResponseCookies,
        config::{CryptoAlgorithm, CryptoRule},
    },
};
use pavex_session::{Session, SessionConfig, finalize_session};

#[tokio::test]
async fn session_cookie_must_be_either_signed_or_encrypted() {
    let (store, config) = (store(), SessionConfig::default());
    let response = Response::ok();
    let mut response_cookies = ResponseCookies::new();
    let processor = ProcessorConfig::default().into();

    let mut session = Session::new(&store, &config, None);
    session.insert("key", "value").await.unwrap();

    let err = finalize_session(response, &mut response_cookies, &processor, session)
        .await
        .unwrap_err();
    assert_snapshot!(err, @"The session cookie (`id`) is not configured to be signed nor encrypted. This is a security risk, as the client-side session state may be intercepted and manipulated by an attacker. Configure the cookie processor to sign or encrypt the session cookie; check out https://docs.rs/biscotti/latest/biscotti/struct.ProcessorConfig.html#structfield.crypto_rules for more information.");
}

#[tokio::test]
async fn non_empty_server_state_works_if_session_cookie_is_signed() {
    let (store, config) = (store(), SessionConfig::default());
    let response = Response::ok();
    let mut response_cookies = ResponseCookies::new();
    let processor = {
        let mut cookie_config = ProcessorConfig::default();
        cookie_config.crypto_rules.push(CryptoRule {
            cookie_names: vec![config.cookie.name.clone()],
            algorithm: CryptoAlgorithm::Signing,
            key: Key::generate(),
            fallbacks: vec![],
        });
        cookie_config.into()
    };

    let mut session = Session::new(&store, &config, None);
    session.insert("key", "value").await.unwrap();

    finalize_session(response, &mut response_cookies, &processor, session)
        .await
        .unwrap();
}

#[tokio::test]
async fn non_empty_server_state_works_if_session_cookie_is_encrypted() {
    let (store, config) = (store(), SessionConfig::default());
    let response = Response::ok();
    let mut response_cookies = ResponseCookies::new();
    let processor = {
        let mut cookie_config = ProcessorConfig::default();
        cookie_config.crypto_rules.push(CryptoRule {
            cookie_names: vec![config.cookie.name.clone()],
            algorithm: CryptoAlgorithm::Encryption,
            key: Key::generate(),
            fallbacks: vec![],
        });
        cookie_config.into()
    };

    let mut session = Session::new(&store, &config, None);
    session.insert("key", "value").await.unwrap();

    finalize_session(response, &mut response_cookies, &processor, session)
        .await
        .unwrap();
}

#[tokio::test]
async fn non_empty_client_state_works_if_session_cookie_is_encrypted() {
    let (store, config) = (store(), SessionConfig::default());
    let response = Response::ok();
    let mut response_cookies = ResponseCookies::new();
    let processor = {
        let mut cookie_config = ProcessorConfig::default();
        cookie_config.crypto_rules.push(CryptoRule {
            cookie_names: vec![config.cookie.name.clone()],
            algorithm: CryptoAlgorithm::Encryption,
            key: Key::generate(),
            fallbacks: vec![],
        });
        cookie_config.into()
    };

    let mut session = Session::new(&store, &config, None);
    session.client_mut().insert("key", "value").unwrap();

    finalize_session(response, &mut response_cookies, &processor, session)
        .await
        .unwrap();
}

#[tokio::test]
async fn session_cookie_cannot_be_plain_if_client_side_state_is_not_empty() {
    let (store, config) = (store(), SessionConfig::default());
    let response = Response::ok();
    let mut response_cookies = ResponseCookies::new();
    let processor = ProcessorConfig::default().into();

    let mut session = Session::new(&store, &config, None);
    session.client_mut().insert("key", "value").unwrap();

    let err = finalize_session(response, &mut response_cookies, &processor, session)
        .await
        .unwrap_err();
    assert_snapshot!(err, @"The client-side session state is not empty, but the session cookie (`id`) is not configured to be encrypted. This may be a security risk, as the client-side session state may be intercepted and read by an attacker. Configure the cookie processor to encrypt the session cookie; check out https://docs.rs/biscotti/latest/biscotti/struct.ProcessorConfig.html#structfield.crypto_rules for more information.");
}

#[tokio::test]
async fn session_cookie_cannot_be_just_signed_if_client_side_state_is_not_empty() {
    let (store, session_config) = (store(), SessionConfig::default());
    let response = Response::ok();
    let mut response_cookies = ResponseCookies::new();
    let processor = {
        let mut cookie_config = ProcessorConfig::default();
        cookie_config.crypto_rules.push(CryptoRule {
            cookie_names: vec![session_config.cookie.name.clone()],
            algorithm: CryptoAlgorithm::Signing,
            key: Key::generate(),
            fallbacks: vec![],
        });
        cookie_config.into()
    };

    let mut session = Session::new(&store, &session_config, None);
    session.client_mut().insert("key", "value").unwrap();

    let err = finalize_session(response, &mut response_cookies, &processor, session)
        .await
        .unwrap_err();
    assert_snapshot!(err, @"The client-side session state is not empty, but the session cookie (`id`) is not configured to be encrypted. This may be a security risk, as the client-side session state may be intercepted and read by an attacker. Configure the cookie processor to encrypt the session cookie; check out https://docs.rs/biscotti/latest/biscotti/struct.ProcessorConfig.html#structfield.crypto_rules for more information.");
}
