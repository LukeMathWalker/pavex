mod private {
    use pavex_macros::pre_process;

    pub struct A;

    #[pre_process(id = "A_")]
    pub fn a() -> A {
        todo!()
    }
}

fn main() {}
