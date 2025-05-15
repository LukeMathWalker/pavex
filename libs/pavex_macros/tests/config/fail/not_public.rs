use pavex_macros::config;

#[config(key = "a")]
struct A;

#[config(key = "a1")]
enum A1 {}

#[config(key = "b")]
pub(crate) struct B;

#[config(key = "b1")]
enum B1 {}

fn main() {}
