use pavex::get;
use pavex::http::StatusCode;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTagsResponse {
    tags: Vec<String>,
}

#[get(path = "/tags")]
pub fn list_tags() -> StatusCode {
    StatusCode::OK
}
