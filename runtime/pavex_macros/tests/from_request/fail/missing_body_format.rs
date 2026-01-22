use pavex_macros::FromRequest;

#[derive(FromRequest)]
pub struct MyStruct {
    #[body]
    b: String,
}

fn main() {}
