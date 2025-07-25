//! px:extracted
use pavex::get;
use pavex::http::StatusCode;
use pavex::request::path::PathParams;

// px:struct_def:start
#[PathParams] // px:struct_def:hl
// px:struct_def_without_attr:start
pub struct GetUserParams {
    pub id: u64, // px:struct_def_without_attr:hl
}
// px:struct_def:end
// px:struct_def_without_attr:end

#[get(path = "/users/{id}")] /* (1)! */
pub fn parsed(params: &PathParams<GetUserParams>) -> StatusCode {
    println!("The user id is {}", params.0.id);
    StatusCode::OK // px::skip
}
