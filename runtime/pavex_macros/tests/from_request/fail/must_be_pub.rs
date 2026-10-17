use pavex_macros::FromRequest;

#[derive(FromRequest)]
struct Private {
    #[path_param]
    a: u64,
}

#[derive(FromRequest)]
pub(crate) struct PubWithRestrictions {
    #[path_param]
    a: u64,
}

fn main() {}
