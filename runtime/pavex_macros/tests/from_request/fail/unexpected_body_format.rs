use pavex_macros::FromRequest;

#[derive(FromRequest)]
pub struct MyStruct {
    #[body(format = "fantasy")]
    b: String,
}

fn main() {}
