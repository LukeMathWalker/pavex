use pavex::get;
use pavex::http::StatusCode;

#[get(path = "/greet")]
pub fn greet() -> StatusCode {
    todo!()
}
