#[derive(serde::Deserialize)]
#[pavex_macros::PathParams]
struct MyStruct {
    field1: i32,
    field2: String,
}

fn main() {}
