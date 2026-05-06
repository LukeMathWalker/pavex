use pavex_macros::FromRequest;

#[derive(FromRequest)]
pub struct MyStruct {
    #[query_param]
    a: NotDeserializable,
}

pub struct NotDeserializable;

fn main() {}
