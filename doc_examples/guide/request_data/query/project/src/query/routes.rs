use pavex::http::StatusCode;
use pavex::request::RequestHead;

pub fn handler(head: &RequestHead) -> StatusCode {
    if let Some(raw_query /* (1)! */) = head.uri.query() {
        println!("The raw query is `{raw_query}`");
    } else {
        println!("There is no query string");
    }
    StatusCode::OK
}
