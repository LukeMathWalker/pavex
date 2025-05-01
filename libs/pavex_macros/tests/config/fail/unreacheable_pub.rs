mod private {
    use pavex_macros::config;

    #[config(key = "a")]
    pub struct A;

    #[config(key = "a1")]
    pub enum A1 {}
}

fn main() {}
