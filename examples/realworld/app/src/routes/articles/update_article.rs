use pavex::{
    http::StatusCode,
    put,
    request::{body::JsonBody, path::PathParams},
};

use crate::schemas::Article;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateArticleBody {
    pub title: Option<String>,
    pub description: Option<String>,
    pub body: Option<String>,
}

#[derive(Debug)]
#[PathParams]
pub struct UpdateArticleRoute {
    pub slug: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateArticleResponse {
    pub article: Article,
}

#[put(path = "/{slug}")]
pub fn update_article(
    _params: PathParams<UpdateArticleRoute>,
    _body: JsonBody<UpdateArticleBody>,
) -> StatusCode {
    StatusCode::OK
}
