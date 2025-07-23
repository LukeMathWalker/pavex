//! px:extraction
use pavex::get;
use pavex::http::StatusCode;
use pavex::request::query::QueryParams;

// px:struct_def:start
#[derive(serde::Deserialize)] // px:struct_def:hl
                              // px:struct_def_without_attr:start
pub struct SearchParams {
    pub sorted: bool, // px:struct_def_without_attr:hl
}
// px:struct_def:end
// px:struct_def_without_attr:end

#[get(path = "/search")]
pub fn search(params: &QueryParams<SearchParams>) -> StatusCode {
    if params.0.sorted {
        println!("The search is sorted");
    }
    StatusCode::OK // px::skip
}
