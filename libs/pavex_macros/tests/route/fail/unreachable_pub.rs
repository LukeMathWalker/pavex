mod private {
    use pavex_macros::get;

    #[get(path = "/a")]
    pub fn a() {
        todo!()
    }
}

fn main() {}
