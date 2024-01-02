use pavex::http::StatusCode;
use pavex::request::RequestHead;

pub fn handler(head: &RequestHead) -> StatusCode {
    println!("The raw path is: {}", head.target.path());
    StatusCode::OK
}
