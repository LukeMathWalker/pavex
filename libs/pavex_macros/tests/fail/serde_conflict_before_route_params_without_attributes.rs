#[derive(serde::Deserialize)]
#[pavex_macros::RouteParams]
struct MyStruct {
    field1: i32,
    field2: String,
}

fn main() {}
