use pavex_macros::from;

fn main() {
    let _ = from![crate, pavex_sqlx, crate::router, super::module, self::inner];
}
