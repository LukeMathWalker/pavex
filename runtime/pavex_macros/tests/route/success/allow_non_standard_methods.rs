use pavex_macros::route;

#[route(path = "/any", allow(any_method, non_standard_methods))]
pub fn any() {}

#[route(path = "/get_ish", method = ["GeT"], allow(non_standard_methods))]
pub fn get_ish() {}

#[route(path = "/multi", method = ["HEY", "GeT"], allow(non_standard_methods))]
pub fn multiple() {}

fn main() {}
