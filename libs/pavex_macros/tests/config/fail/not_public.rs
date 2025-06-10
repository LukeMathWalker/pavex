use pavex_macros::config;

#[config(key = "a")]
struct A;

#[config(key = "a1")]
enum A1 {}

#[config(key = "b")]
pub(crate) struct B;

#[config(key = "b1")]
enum B1 {}

#[config(key = "a2")]
use sub::A as A2;

#[config(key = "a3")]
pub(crate) use sub::A as A3;

mod sub {
    pub struct A;
}

fn main() {}
