use pavex_macros::FromRequest;

#[derive(FromRequest)]
pub struct MyStruct {
    #[path_params(name = "a1")]
    a: u64,

    #[query_params(name = "b1")]
    b: String,

    #[headers(name = "c1")]
    c: String,
}

fn main() {}
