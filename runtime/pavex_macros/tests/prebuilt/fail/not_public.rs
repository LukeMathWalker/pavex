use pavex_macros::prebuilt;

#[prebuilt]
struct A;

#[prebuilt]
enum A1 {}

#[prebuilt]
pub(crate) struct B;

#[prebuilt]
enum B1 {}

fn main() {}
