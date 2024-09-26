use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;

pub fn handler(_a: my_mod::A<my_mod::B>) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    my_mod::register(&mut bp);
    bp.route(GET, "/home", f!(self::handler));
    bp
}

pub mod my_mod {
    use pavex::blueprint::{constructor::Lifecycle, Blueprint};
    use pavex::f;

    pub fn register(bp: &mut Blueprint) {
        bp.constructor(f!(self::A::<self::B>::new), Lifecycle::RequestScoped);
    }

    pub struct A<T>(T);

    impl<T> A<T> {
        pub fn new() -> A<T> {
            todo!()
        }
    }

    pub struct B;
}
