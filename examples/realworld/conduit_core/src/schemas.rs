//! A collection of typed schemas that are used across multiple
//! routes as a fragment of the incoming request or the returned response.

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Article {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub body: String,
    pub tag_list: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub favorited: bool,
    pub favorites_count: u64,
    pub author: Profile,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub id: u64,
    pub created_at: String,
    pub updated_at: String,
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
    pub token: String,
    pub username: String,
    pub bio: String,
    pub image: String,
}
