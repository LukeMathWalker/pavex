/// A JSON API error object.
/// See <https://jsonapi.org/examples/#error-objects> for a quick reference.
#[derive(serde::Deserialize)]
pub struct JsonApiErrors {
    pub errors: Vec<JsonApiError>,
}

#[derive(serde::Deserialize)]
pub struct JsonApiError {
    pub code: String,
    pub detail: Option<String>,
}
