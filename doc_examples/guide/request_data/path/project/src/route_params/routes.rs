use pavex::http::StatusCode;
use pavex::request::route::RouteParams;

#[RouteParams]
pub struct GetUserParams {
    pub id: u64,
}

pub fn handler(params: &RouteParams<GetUserParams>) -> StatusCode {
    println!("The user id is {}", params.0.id);
    StatusCode::OK
}
