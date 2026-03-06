use pavex_macros::FromRequest;

#[derive(FromRequest)]
pub struct MyStruct {
    a: u64,
}

fn main() {}
