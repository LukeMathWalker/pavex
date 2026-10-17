use pavex_macros::FromRequest;

#[derive(FromRequest)]
pub struct One<T> {
    #[path_param]
    a: T,
}

#[derive(FromRequest)]
pub struct Three<T, S, R> {
    #[path_param]
    a: T,
    #[path_param]
    b: S,
    #[path_param]
    c: R,
}

fn main() {}
