mod private {
    use pavex_macros::prebuilt;

    #[prebuilt(id = "A_")]
    pub struct A;

    #[prebuilt]
    pub enum A1 {}
}

fn main() {}
