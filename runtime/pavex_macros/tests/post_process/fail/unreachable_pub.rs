mod private {
    use pavex_macros::post_process;

    pub struct A;

    #[post_process(id = "A_")]
    pub fn a() -> A {
        todo!()
    }
}

fn main() {}
