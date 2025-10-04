#[pavex_macros::PathParams]
#[derive(serde::Deserialize)]
struct MyStruct {
    field1: i32,
    field2: String,
}

fn main() {}
