use pavex_macros::route;

#[route(path = "/one", method = "HEY")]
pub fn one() {}

#[route(path = "/multi", method = ["HEY", "GET"])]
pub fn multiple() {}

#[route(path = "/multi_invalid", method = ["HEY", "CuStOm"])]
pub fn multiple_invalid() {}

fn main() {}
