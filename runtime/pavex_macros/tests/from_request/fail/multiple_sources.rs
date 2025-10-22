use pavex_macros::FromRequest;

#[derive(FromRequest)]
pub struct MyStruct {
    // Path param
    #[path_param]
    #[path_params]
    a: u64,
}

fn main() {}
