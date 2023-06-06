use pavex_builder::constructor::Lifecycle;
use pavex_builder::router::{DELETE, POST, PUT};
use pavex_builder::{f, router::GET, Blueprint};
use pavex_runtime::extract::body::{JsonBody, BodySizeLimit};
use pavex_runtime::extract::query::QueryParams;
use pavex_runtime::extract::route::RouteParams;
use pavex_runtime::hyper::StatusCode;

pub fn body_size_limit() -> BodySizeLimit {
    BodySizeLimit::Enabled {
        max_n_bytes: 10
    }
}

pub(crate) fn articles_bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::articles::body_size_limit), Lifecycle::RequestScoped);

    bp.route(GET, "", f!(crate::articles::list_articles));
    bp.route(POST, "", f!(crate::articles::publish_article));
    bp.route(GET, "/feed", f!(crate::articles::get_feed));
    bp.route(GET, "/:slug", f!(crate::articles::get_article));
    bp.route(DELETE, "/:slug", f!(crate::articles::delete_article));
    bp.route(PUT, "/:slug", f!(crate::articles::update_article));
    bp
}

#[derive(Debug, serde::Deserialize)]
pub struct GetFeed {
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default = "default_offset")]
    pub offset: u64,
}

#[derive(Debug, serde::Deserialize)]
pub struct GetArticles {
    pub tag: Option<String>,
    pub author: Option<String>,
    pub favorited: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default = "default_offset")]
    pub offset: u64,
}

#[derive(Debug, serde::Deserialize)]
pub struct PublishArticle {
    pub title: String,
    pub description: String,
    pub body: String,
    #[serde(rename = "tagList", default)]
    pub tag_list: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct UpdateArticleBody {
    pub title: Option<String>,
    pub description: Option<String>,
    pub body: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct UpdateArticleRoute {
    pub slug: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct GetArticle {
    pub slug: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct DeleteArticle {
    pub slug: String,
}

fn default_limit() -> u64 {
    20
}

fn default_offset() -> u64 {
    0
}

pub fn list_articles(_params: QueryParams<GetArticles>) -> StatusCode {
    StatusCode::OK
}

pub fn get_feed(_params: QueryParams<GetFeed>) -> StatusCode {
    StatusCode::OK
}

pub fn get_article(_params: RouteParams<GetArticle>) -> StatusCode {
    StatusCode::OK
}

pub fn publish_article(_body: JsonBody<PublishArticle>) -> StatusCode {
    StatusCode::OK
}

pub fn delete_article(_params: RouteParams<DeleteArticle>) -> StatusCode {
    StatusCode::OK
}

pub fn update_article(
    _params: RouteParams<UpdateArticleRoute>,
    _body: JsonBody<UpdateArticleBody>,
) -> StatusCode {
    StatusCode::OK
}
