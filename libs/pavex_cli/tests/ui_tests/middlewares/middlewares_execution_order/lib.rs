use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::{f, t};
use pavex::{request::path::PathParams, response::Response};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub struct Spy(Arc<Mutex<Vec<String>>>);

impl Spy {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }

    pub async fn push(&self, s: String) {
        self.0.lock().await.push(s);
    }

    pub async fn get(&self) -> Vec<String> {
        self.0.lock().await.clone()
    }
}

pub async fn handler(spy: &Spy) -> Response {
    spy.push("handler".to_string()).await;
    Response::ok()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.prebuilt(t!(self::Spy));
    bp.nest(top_level());
    bp.nest(after_handler());
    bp.nest(early_return());
    bp.nest(nested());
    bp
}

pub fn top_level() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.wrap(f!(crate::first));
    bp.wrap(f!(crate::second));
    bp.post_process(f!(crate::first_post));
    bp.post_process(f!(crate::second_post));
    bp.pre_process(f!(crate::first_pre));
    bp.pre_process(f!(crate::second_pre));
    bp.route(GET, "/top_level", f!(crate::handler));
    bp
}

pub fn after_handler() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.wrap(f!(crate::first));
    bp.post_process(f!(crate::first_post));
    bp.pre_process(f!(crate::first_pre));
    bp.route(GET, "/after_handler", f!(crate::handler));
    bp.wrap(f!(crate::second));
    bp.pre_process(f!(crate::second_pre));
    bp.post_process(f!(crate::second_post));
    bp
}

pub fn early_return() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.wrap(f!(crate::first));
    bp.post_process(f!(crate::first_post));
    bp.pre_process(f!(crate::early_return_pre));
    bp.wrap(f!(crate::second));
    bp.pre_process(f!(crate::second_pre));
    bp.post_process(f!(crate::second_post));
    bp.route(GET, "/early_return", f!(crate::handler));
    bp
}

pub fn nested() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.wrap(f!(crate::first));
    bp.pre_process(f!(crate::first_pre));
    bp.post_process(f!(crate::first_post));
    bp.nest({
        let mut bp = Blueprint::new();
        bp.wrap(f!(crate::second));
        bp.post_process(f!(crate::second_post));
        bp.pre_process(f!(crate::second_pre));
        bp.route(GET, "/nested", f!(crate::handler));
        bp
    });
    bp.wrap(f!(crate::third));
    bp.pre_process(f!(crate::third_pre));
    bp.post_process(f!(crate::third_post));
    bp
}

macro_rules! spy_mw {
    ($name:ident) => {
        pub async fn $name<C>(
            spy: &$crate::Spy,
            next: pavex::middleware::Next<C>,
        ) -> pavex::response::Response
        where
            C: std::future::IntoFuture<Output = pavex::response::Response>,
        {
            spy.push(format!("{} - start", stringify!($name))).await;
            let response = next.await;
            spy.push(format!("{} - end", stringify!($name))).await;
            response
        }
    };
}

spy_mw!(first);
spy_mw!(second);
spy_mw!(third);

macro_rules! spy_post {
    ($name:ident) => {
        pub async fn $name(
            spy: &$crate::Spy,
            response: pavex::response::Response,
        ) -> pavex::response::Response {
            spy.push(format!("{}", stringify!($name))).await;
            response
        }
    };
}

spy_post!(first_post);
spy_post!(second_post);
spy_post!(third_post);

pub async fn early_return_pre(spy: &Spy) -> pavex::middleware::Processing {
    spy.push("early_return_pre".to_string()).await;
    pavex::middleware::Processing::EarlyReturn(Response::ok())
}

macro_rules! spy_pre {
    ($name:ident) => {
        pub async fn $name(spy: &$crate::Spy) -> pavex::middleware::Processing {
            spy.push(format!("{}", stringify!($name))).await;
            pavex::middleware::Processing::Continue
        }
    };
}

spy_pre!(first_pre);
spy_pre!(second_pre);
spy_pre!(third_pre);
