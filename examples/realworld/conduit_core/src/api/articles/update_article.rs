use pavex::{
    extract::{body::JsonBody, route::RouteParams},
    hyper::StatusCode,
};

#[derive(Debug, serde::Deserialize)]
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

pub fn update_article(
    _params: RouteParams<UpdateArticleRoute>,
    _body: JsonBody<UpdateArticleBody>,
) -> StatusCode {
    StatusCode::OK
}
