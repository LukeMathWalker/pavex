#[derive(serde::Deserialize)]
#[pavex_macros::PathParams]
#[serde(rename_all = "camelCase")]
struct MyStruct {
    #[serde(rename = "field1")]
    field1: i32,
    field2: String,
}

fn main() {}
