pub fn handler(head: &RequestHead) -> Result<Response, LoginError /* (1)! */> {
    // Handler logic...
    todo!()
}

use crate::universal::LoginError;
use pavex::request::RequestHead;
use pavex::response::Response;
