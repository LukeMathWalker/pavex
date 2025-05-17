use pavex_macros::route;

#[route(path = "/single", method = "GET", allow(any_method))]
pub fn single() {}

#[route(path = "/multiple", method = ["GET", "POST"], allow(any_method))]
pub fn multiple() {}

fn main() {}
