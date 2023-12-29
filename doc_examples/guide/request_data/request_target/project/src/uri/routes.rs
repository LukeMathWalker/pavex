use pavex::http::StatusCode;
use pavex::request::RequestHead;

pub fn handler(head: &RequestHead) -> StatusCode {
    println!("The request target is {}", head.uri);
    StatusCode::OK
}
