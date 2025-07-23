//! px:authentication
use pavex::middleware::Processing;
use pavex::pre_process;
use pavex::response::Response;

use crate::User;

#[pre_process]
pub fn reject_anonymous(user: &User) -> Processing {
    if let User::Anonymous = user {
        let r = Response::unauthorized();
        Processing::EarlyReturn(r)
    } else {
        Processing::Continue
    }
}
