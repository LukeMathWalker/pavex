use pavex::http::StatusCode;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTagsResponse {
    tags: Vec<String>
}

pub fn get_tags() -> StatusCode {
    StatusCode::OK
}
