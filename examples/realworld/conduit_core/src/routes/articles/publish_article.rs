use pavex::{extract::body::JsonBody, hyper::StatusCode};

use crate::schemas::Article;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishArticle {
    pub title: String,
    pub description: String,
    pub body: String,
    #[serde(rename = "tagList", default)]
    pub tag_list: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishArticleResponse {
    pub article: Article,
}

pub fn publish_article(_body: JsonBody<PublishArticle>) -> StatusCode {
    StatusCode::OK
}
