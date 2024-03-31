use pavex::middleware::Processing;
use pavex::response::Response;

use crate::user::User;

pub fn reject_anonymous(user: &User) -> Processing
{
    if let User::Anonymous = user {
        let r = Response::unauthorized();
        Processing::EarlyReturn(r)
    } else {
        Processing::Continue
    }
}
