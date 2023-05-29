use pavex_builder::constructor::Lifecycle;
use pavex_builder::{f, router::GET, Blueprint};
use pavex_runtime::extract::query::QueryParams;
use pavex_runtime::hyper::StatusCode;

pub(crate) fn articles_bp() -> Blueprint {
    let mut bp = Blueprint::new();

    bp.constructor(
        f!(pavex_runtime::extract::query::QueryParams::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(
        pavex_runtime::extract::query::errors::ExtractQueryParamsError::into_response
    ));

    bp.route(GET, "/", f!(crate::articles::list_articles));
    bp
}

#[derive(Debug, serde::Deserialize)]
pub struct GetArticlesApiRequest {
    pub tag: Option<String>,
    pub author: Option<String>,
    pub favorited: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default = "default_offset")]
    pub offset: u64,
}

fn default_limit() -> u64 {
    20
}

fn default_offset() -> u64 {
    0
}

pub fn list_articles(_params: QueryParams<GetArticlesApiRequest>) -> StatusCode {
    StatusCode::OK
}
