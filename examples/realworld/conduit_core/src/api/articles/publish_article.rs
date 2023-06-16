use pavex::{extract::body::JsonBody, hyper::StatusCode};

#[derive(Debug, serde::Deserialize)]
pub struct PublishArticle {
    pub title: String,
    pub description: String,
    pub body: String,
    #[serde(rename = "tagList", default)]
    pub tag_list: Vec<String>,
}

pub fn publish_article(_body: JsonBody<PublishArticle>) -> StatusCode {
    StatusCode::OK
}
