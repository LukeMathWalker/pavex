pub fn handler(head: &RequestHead) -> Result<Response, LoginError /* (1)! */> {
    // Handler logic...
    todo!()
}

use crate::core::error_handler::LoginError;
use pavex::request::RequestHead;
use pavex::response::Response;
