// px:fallible:start
#[pavex::post(path = "/login")]
pub fn login(head: &RequestHead) -> Result<Response, LoginError /* px::ann:1 */> {
    todo!() // px::skip
}
// px:fallible:end

use crate::login::LoginError;
use pavex::request::RequestHead;
use pavex::response::Response;
