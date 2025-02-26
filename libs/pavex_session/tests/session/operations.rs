//! Tests looking at the interaction between different operations on the session state.
use crate::fixtures::store;
use googletest::{
    assert_that,
    prelude::{eq, none},
};
use pavex_session::{Session, SessionConfig};

#[tokio::test]
async fn fresh_session_is_empty() {
    let (store, config) = (store(), SessionConfig::default());

    let session = Session::new(&store, &config, None);
    assert!(!session.is_invalidated());
    assert!(session.client().is_empty());
    assert!(
        session
            .server()
            .is_empty()
            .await
            .expect("Failed to load session state")
    );

    // Trying to get a non-existing key on a fresh session returns `None`
    let key = "key".to_string();

    assert!(session.client().get::<String>(&key).unwrap().is_none());
    assert!(session.client().get_value(&key).is_none());
    assert!(
        session
            .server()
            .get::<String>(&key)
            .await
            .unwrap()
            .is_none()
    );
    assert!(session.server().get_value(&key).await.unwrap().is_none());
}

#[tokio::test]
async fn operation_outcomes_are_immediately_visible() {
    let (store, config) = (store(), SessionConfig::default());

    let mut session = Session::new(&store, &config, None);

    let key = "key".to_string();
    let client_value = "hey";
    let server_value = "yo";

    session.client_mut().set(key.clone(), client_value).unwrap();
    session
        .server_mut()
        .set(key.clone(), server_value)
        .await
        .unwrap();

    let stored_client_value: String = session.client().get(&key).unwrap().unwrap();
    let stored_server_value: String = session.server().get(&key).await.unwrap().unwrap();

    // Even though we used the same key, the client-side and server-side states
    // don't overwrite each other. They're completely separate bags of values.
    assert_that!(&stored_client_value, eq(&client_value));
    assert_that!(&stored_server_value, eq(&server_value));

    // Internal consistency
    assert_that!(
        &stored_client_value,
        eq(&session.client_mut().get::<String>(&key).unwrap().unwrap())
    );
    assert_that!(
        &stored_server_value,
        eq(&session
            .server_mut()
            .get::<String>(&key)
            .await
            .unwrap()
            .unwrap())
    );

    // We can also avoid the deserialize step by using `get_value`.
    let stored_client_value = session.client().get_value(&key).unwrap().to_owned();
    let stored_server_value = session
        .server()
        .get_value(&key)
        .await
        .unwrap()
        .unwrap()
        .to_owned();

    assert_that!(
        stored_client_value,
        eq(&serde_json::Value::String(client_value.into()))
    );
    assert_that!(
        stored_server_value,
        eq(&serde_json::Value::String(server_value.into()))
    );
    // Internal consistency
    assert_that!(
        stored_client_value,
        eq(session.client_mut().get_value(&key).unwrap())
    );
    assert_that!(
        stored_server_value,
        eq(session.server_mut().get_value(&key).await.unwrap().unwrap())
    );

    // The session is now reported as being non-empty
    assert_that!(session.client().is_empty(), eq(false));
    assert_that!(session.client_mut().is_empty(), eq(false));
    assert_that!(session.server().is_empty().await.unwrap(), eq(false));
    assert_that!(session.server_mut().is_empty().await.unwrap(), eq(false));

    session.client_mut().remove::<String>(&key).unwrap();
    session.server_mut().remove::<String>(&key).await.unwrap();

    assert_that!(session.client().get_value(&key), none());
    assert_that!(session.client_mut().get_value(&key), none());
    assert_that!(session.server().get_value(&key).await.unwrap(), none());
    assert_that!(session.server_mut().get_value(&key).await.unwrap(), none());

    assert_that!(session.client().is_empty(), eq(true));
    assert_that!(session.client_mut().is_empty(), eq(true));
    assert_that!(session.server().is_empty().await.unwrap(), eq(true));
    assert_that!(session.server_mut().is_empty().await.unwrap(), eq(true));
}

#[tokio::test]
async fn server_set_overwrites_previous_values() {
    let (store, config) = (store(), SessionConfig::default());

    let mut session = Session::new(&store, &config, None);

    let key = "key".to_string();
    let value1 = "yo";
    let value2 = "yo";

    session.server_mut().set(key.clone(), value1).await.unwrap();

    let stored_value: String = session.server().get(&key).await.unwrap().unwrap();
    assert_that!(&stored_value, eq(&value1));

    session.server_mut().set(key.clone(), value2).await.unwrap();

    let stored_value: String = session.server().get(&key).await.unwrap().unwrap();
    assert_that!(&stored_value, eq(&value2));
}

#[tokio::test]
async fn client_set_overwrites_previous_values() {
    let (store, config) = (store(), SessionConfig::default());

    let mut session = Session::new(&store, &config, None);

    let key = "key".to_string();
    let value1 = "yo";
    let value2 = "yo";

    session.client_mut().set(key.clone(), value1).unwrap();

    let stored_value: String = session.client().get(&key).unwrap().unwrap();
    assert_that!(&stored_value, eq(&value1));

    session.client_mut().set(key.clone(), value2).unwrap();

    let stored_value: String = session.client().get(&key).unwrap().unwrap();
    assert_that!(&stored_value, eq(&value2));
}

#[tokio::test]
async fn client_get_fails_if_deserialization_fails() {
    let (store, config) = (store(), SessionConfig::default());
    let mut session = Session::new(&store, &config, None);

    let key = "key".to_string();
    session.client_mut().set(key.clone(), "yo").unwrap();
    let err = session.client().get::<u64>(&key).unwrap_err();
    assert_eq!(
        err.to_string(),
        "Failed to deserialize the value associated with `key` in the client-side session state"
    );
}

#[tokio::test]
async fn server_get_fails_if_deserialization_fails() {
    let (store, config) = (store(), SessionConfig::default());
    let mut session = Session::new(&store, &config, None);

    let key = "key".to_string();
    session.server_mut().set(key.clone(), "yo").await.unwrap();
    let err = session.server().get::<u64>(&key).await.unwrap_err();
    assert_eq!(
        err.to_string(),
        "Failed to deserialize the value associated with `key` in the server-side session state"
    );
}

#[tokio::test]
async fn client_remove_fails_if_deserialization_fails() {
    let (store, config) = (store(), SessionConfig::default());
    let mut session = Session::new(&store, &config, None);

    let key = "key".to_string();
    session.client_mut().set(key.clone(), "yo").unwrap();
    let err = session.client_mut().remove::<u64>(&key).unwrap_err();
    assert_eq!(
        err.to_string(),
        "Failed to deserialize the value associated with `key` in the client-side session state"
    );
}

#[tokio::test]
async fn server_remove_fails_if_deserialization_fails() {
    let (store, config) = (store(), SessionConfig::default());
    let mut session = Session::new(&store, &config, None);

    let key = "key".to_string();
    session.server_mut().set(key.clone(), "yo").await.unwrap();
    let err = session.server_mut().remove::<u64>(&key).await.unwrap_err();
    assert_eq!(
        err.to_string(),
        "Failed to deserialize the value associated with `key` in the server-side session state"
    );
}

// A type that can't be serialized.
struct Unserializable;

impl serde::Serialize for Unserializable {
    fn serialize<S>(&self, _: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Err(serde::ser::Error::custom("Failed to serialize value"))
    }
}

#[tokio::test]
async fn server_set_fails_if_serialization_fails() {
    let (store, config) = (store(), SessionConfig::default());
    let mut session = Session::new(&store, &config, None);

    let err = session
        .server_mut()
        .set("key".into(), Unserializable)
        .await
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "Failed to serialize the value that would have been associated with `key` in the server-side session state"
    );
}

#[tokio::test]
async fn client_set_fails_if_serialization_fails() {
    let (store, config) = (store(), SessionConfig::default());
    let mut session = Session::new(&store, &config, None);

    let err = session
        .client_mut()
        .set("key".into(), Unserializable)
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "Failed to serialize the value that would have been associated with `key` in the client-side session state"
    );
}

#[tokio::test]
async fn clearing_an_empty_session_does_not_error() {
    let (store, config) = (store(), SessionConfig::default());

    let mut session = Session::new(&store, &config, None);

    session.client_mut().clear();
    session.server_mut().clear().await.unwrap();

    assert!(session.client().is_empty());
    assert!(session.server().is_empty().await.unwrap());
}

#[tokio::test]
async fn clearing_empties_the_session() {
    let (store, config) = (store(), SessionConfig::default());

    let mut session = Session::new(&store, &config, None);

    session.client_mut().set("client.key".into(), 12).unwrap();
    session.client_mut().set("client.key2".into(), 21).unwrap();
    session
        .server_mut()
        .set("server.key".into(), 43)
        .await
        .unwrap();
    session
        .server_mut()
        .set("server.key2".into(), "Message")
        .await
        .unwrap();

    assert!(!session.client().is_empty());
    assert!(!session.server().is_empty().await.unwrap());

    session.client_mut().clear();

    // Only the client-side session is emptied.
    assert!(session.client().is_empty());
    assert!(!session.server().is_empty().await.unwrap());

    session.server_mut().clear().await.unwrap();

    // Now they're both empty.
    assert!(session.client().is_empty());
    assert!(session.server().is_empty().await.unwrap());
}

#[tokio::test]
async fn removing_a_non_existing_key_does_not_error() {
    let (store, config) = (store(), SessionConfig::default());

    let mut session = Session::new(&store, &config, None);

    let key = "my_key";

    let removed: Option<String> = session.client_mut().remove(&key).unwrap();
    assert_that!(removed, none());
    let removed: Option<String> = session.server_mut().remove(&key).await.unwrap();
    assert_that!(removed, none());

    assert_that!(session.client_mut().remove_value(&key), none());
    assert_that!(
        session.server_mut().remove_value(&key).await.unwrap(),
        none()
    );
}

#[tokio::test]
async fn operations_on_an_invalidated_session_are_noops() {
    let (store, config) = (store(), SessionConfig::default());
    let mut session = Session::new(&store, &config, None);

    let key = "my_key";
    session.client_mut().set(key.to_owned(), "hey").unwrap();
    session
        .server_mut()
        .set(key.to_owned(), "yo")
        .await
        .unwrap();

    session.invalidate();

    assert!(session.is_invalidated());

    // The session is reported as being empty, immediately
    assert!(session.client().is_empty());
    assert!(session.server().is_empty().await.unwrap());

    // Removals are no-ops
    let removed: Option<String> = session.client_mut().remove(key).unwrap();
    assert_that!(removed, none());
    let removed: Option<String> = session.server_mut().remove(key).await.unwrap();
    assert_that!(removed, none());

    assert_that!(session.client_mut().remove_value(key), none());
    assert_that!(
        session.server_mut().remove_value(key).await.unwrap(),
        none()
    );

    // Insertions are no-ops
    session.client_mut().set(key.to_owned(), "hey").unwrap();
    assert_that!(session.client().get_value(key), none());

    session
        .server_mut()
        .set(key.to_owned(), "yo")
        .await
        .unwrap();
    assert_that!(session.server().get_value(key).await.unwrap(), none());

    // Clears are no-ops
    session.client_mut().clear();
    session.server_mut().clear().await.unwrap();
}

#[tokio::test]
async fn client_get_methods_on_mut_and_non_mut_are_equivalent() {
    let (store, config) = (store(), SessionConfig::default());

    let mut session = Session::new(&store, &config, None);

    let key = "key".to_string();
    let value = "hey";

    session.client_mut().set(key.clone(), value).unwrap();

    let stored_value: String = session.client().get(&key).unwrap().unwrap();
    assert_that!(&stored_value, eq(&value));
    let stored_value: String = session.client_mut().get(&key).unwrap().unwrap();
    assert_that!(&stored_value, eq(&value));

    session.client_mut().remove::<String>(&key).unwrap();

    assert_that!(session.client().get_value(&key), none());
    assert!(session.client().is_empty());
    assert_that!(session.client_mut().get_value(&key), none());
    assert!(session.client_mut().is_empty());
}

#[tokio::test]
async fn server_get_methods_on_mut_and_non_mut_are_equivalent() {
    let (store, config) = (store(), SessionConfig::default());

    let mut session = Session::new(&store, &config, None);

    let key = "key".to_string();
    let value = "hey";

    session.server_mut().set(key.clone(), value).await.unwrap();

    let stored_value: String = session.server().get(&key).await.unwrap().unwrap();
    assert_eq!(&stored_value, value);
    let stored_value: String = session.server_mut().get(&key).await.unwrap().unwrap();
    assert_eq!(&stored_value, value);

    session.server_mut().remove::<String>(&key).await.unwrap();

    assert!(session.server().get_value(&key).await.unwrap().is_none());
    assert!(session.server().is_empty().await.unwrap());
    assert!(
        session
            .server_mut()
            .get_value(&key)
            .await
            .unwrap()
            .is_none()
    );
    assert!(session.server_mut().is_empty().await.unwrap());
}
