use pavex_macros::FromRequest;

#[derive(FromRequest)]
pub struct MyStruct {
    // Conflicting sources.
    #[path_param]
    #[path_params]
    a: u64,

    // Missing source.
    b: String,
}

fn main() {}
