use pavex::blueprint::Blueprint;
pub struct A<T>(T);

impl<T> A<T> {
    pub fn new() -> A<T> {
        todo!()
    }
}

pub struct B;

pub fn handler(_a: A<B>) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    my_mod::register(&mut bp);
    bp
}

pub mod my_mod {
    use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
    use pavex::f;

    pub fn register(bp: &mut Blueprint) {
        bp.constructor(f!(super::A::<super::B>::new), Lifecycle::RequestScoped);
        bp.route(GET, "/home", f!(super::handler));
    }
}
