use pavex::{
    extract::{body::JsonBody, route::RouteParams},
    hyper::StatusCode,
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
#[RouteParams]
pub struct UpdateArticleRoute {
    pub slug: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateArticleResponse {
    pub article: Article,
}

pub fn update_article(
    _params: RouteParams<UpdateArticleRoute>,
    _body: JsonBody<UpdateArticleBody>,
) -> StatusCode {
    StatusCode::OK
}
