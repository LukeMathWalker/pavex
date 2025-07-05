//! px:raw_query
use pavex::get;
use pavex::http::StatusCode;
use pavex::request::RequestHead;

#[get(path = "/raw_query")]
pub fn raw_query(head: &RequestHead) -> StatusCode {
    if let Some(raw_query /* (1)! */) = head.target.query() {
        println!("The raw query is `{raw_query}`");
    } else {
        println!("There is no query string");
    }
    StatusCode::OK // px::skip
}
