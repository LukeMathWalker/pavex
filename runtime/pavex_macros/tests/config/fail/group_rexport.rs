use pavex_macros::config;

#[config(key = "b")]
pub use private::{A, B};

#[config(key = "a")]
pub use private::*;

mod private {
    pub struct A;
    pub struct B;
}

fn main() {}
