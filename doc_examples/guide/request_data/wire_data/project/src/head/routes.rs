use pavex::http::StatusCode;
use pavex::request::RequestHead;

pub fn handler(head: &RequestHead) -> StatusCode {
    println!("The HTTP method is {}", head.method);
    println!("The HTTP version is {:?}", head.version);
    println!("The request target is {}", head.uri.path());
    println!("There are {} headers", head.headers.len());
    StatusCode::OK
}
