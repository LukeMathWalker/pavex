//! A collection of typed schemas that are used across multiple
//! routes as a fragment of the incoming request or the returned response.

use secrecy::{ExposeSecret, Secret};
use time::OffsetDateTime;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Article {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub body: String,
    pub tag_list: Vec<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub favorited: bool,
    pub favorites_count: u64,
    pub author: Profile,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub id: u64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub body: String,
    pub author: Profile,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub username: String,
    pub bio: String,
    pub image: String,
    pub following: bool,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub email: String,
    #[serde(serialize_with = "serialize_secret")]
    pub token: Secret<String>,
    pub username: String,
    pub bio: String,
    pub image: String,
}

/// By default, `Secret<String>` cannot be serialized to prevent accidental
/// exfiltration of sensitive data.
/// This function (and the `serialize_with` attribute) allow us to
/// be explicit when we want to override this behaviour and serialize
/// a sensitive value with `serde`.
fn serialize_secret<S>(secret: &Secret<String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&secret.expose_secret())
}
