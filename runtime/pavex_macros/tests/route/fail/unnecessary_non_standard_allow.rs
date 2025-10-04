use pavex_macros::route;

#[route(path = "/single", method = "GET", allow(non_standard_methods))]
pub fn single() {}

#[route(path = "/multiple", method = ["GET", "POST"], allow(non_standard_methods))]
pub fn multiple() {}

fn main() {}
