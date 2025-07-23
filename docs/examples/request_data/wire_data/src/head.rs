//! px:head
use pavex::get;
use pavex::http::StatusCode;
use pavex::request::RequestHead;

#[get(path = "/head")]
pub fn request_head(head: &RequestHead /* px::ann:1 */) -> StatusCode {
    println!("The HTTP method is {}", head.method);
    println!("The HTTP version is {:?}", head.version);
    println!("The request target is {}", head.target);
    println!("There are {} headers", head.headers.len());
    StatusCode::OK // px::skip
}
