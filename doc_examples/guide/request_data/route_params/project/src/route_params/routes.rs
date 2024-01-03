use pavex::http::StatusCode;
use pavex::request::path::PathParams;

#[PathParams]
pub struct GetUserParams {
    pub id: u64,
}

pub fn handler(params: &PathParams<GetUserParams>) -> StatusCode {
    println!("The user id is {}", params.0.id);
    StatusCode::OK
}
