use pavex::blueprint::{Blueprint, from};
use pavex::response::Response;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
#[pavex::prebuilt]
pub struct Spy(Arc<Mutex<Vec<String>>>);

impl Default for Spy {
    fn default() -> Self {
        Self::new()
    }
}

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

#[pavex::get(path = "/top_level")]
pub async fn top_level_handler(spy: &Spy) -> Response {
    spy.push("handler".to_string()).await;
    Response::ok()
}

#[pavex::get(path = "/after_handler")]
pub async fn after_handler_handler(spy: &Spy) -> Response {
    spy.push("handler".to_string()).await;
    Response::ok()
}

#[pavex::get(path = "/early_return")]
pub async fn early_return_handler(spy: &Spy) -> Response {
    spy.push("handler".to_string()).await;
    Response::ok()
}

#[pavex::get(path = "/failing_pre")]
pub async fn failing_pre_handler(spy: &Spy) -> Response {
    spy.push("handler".to_string()).await;
    Response::ok()
}

#[pavex::get(path = "/nested")]
pub async fn nested_handler(spy: &Spy) -> Response {
    spy.push("handler".to_string()).await;
    Response::ok()
}


pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.nest(top_level());
    bp.nest(after_handler());
    bp.nest(early_return());
    bp.nest(failing_pre());
    bp.nest(nested());
    bp
}

pub fn top_level() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.wrap(FIRST);
    bp.wrap(SECOND);
    bp.post_process(FIRST_POST);
    bp.post_process(SECOND_POST);
    bp.pre_process(FIRST_PRE);
    bp.pre_process(SECOND_PRE);
    bp.route(TOP_LEVEL_HANDLER);
    bp
}

pub fn after_handler() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.wrap(FIRST);
    bp.post_process(FIRST_POST);
    bp.pre_process(FIRST_PRE);
    bp.route(AFTER_HANDLER_HANDLER);
    bp.wrap(SECOND);
    bp.pre_process(SECOND_PRE);
    bp.post_process(SECOND_POST);
    bp
}

pub fn early_return() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.wrap(FIRST);
    bp.post_process(FIRST_POST);
    bp.pre_process(EARLY_RETURN_PRE);
    bp.wrap(SECOND);
    bp.pre_process(SECOND_PRE);
    bp.post_process(SECOND_POST);
    bp.route(EARLY_RETURN_HANDLER);
    bp
}

pub fn failing_pre() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.wrap(FIRST);
    bp.post_process(FIRST_POST);
    bp.pre_process(FAILING_PRE).error_handler(E_500);
    bp.wrap(SECOND);
    bp.pre_process(SECOND_PRE);
    bp.post_process(SECOND_POST);
    bp.route(FAILING_PRE_HANDLER);
    bp
}

pub fn nested() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.wrap(FIRST);
    bp.pre_process(FIRST_PRE);
    bp.post_process(FIRST_POST);
    bp.nest({
        let mut bp = Blueprint::new();
        bp.wrap(SECOND);
        bp.post_process(SECOND_POST);
        bp.pre_process(SECOND_PRE);
        bp.route(NESTED_HANDLER);
        bp
    });
    bp.wrap(THIRD);
    bp.pre_process(THIRD_PRE);
    bp.post_process(THIRD_POST);
    bp
}

macro_rules! spy_mw {
    ($name:ident) => {
        #[pavex::wrap]
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
        #[pavex::post_process]
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

#[pavex::pre_process]
pub async fn early_return_pre(spy: &Spy) -> pavex::middleware::Processing {
    spy.push("early_return_pre".to_string()).await;
    pavex::middleware::Processing::EarlyReturn(Response::ok())
}

#[pavex::pre_process]
pub async fn failing_pre_(spy: &Spy) -> Result<pavex::middleware::Processing, pavex::Error> {
    spy.push("failing_pre".to_string()).await;
    Err(pavex::Error::new("failing_pre"))
}

#[pavex::error_handler]
pub fn e500(_e: &pavex::Error) -> Response {
    Response::internal_server_error()
}

macro_rules! spy_pre {
    ($name:ident) => {
        #[pavex::pre_process]
        pub async fn $name(spy: &$crate::Spy) -> pavex::middleware::Processing {
            spy.push(format!("{}", stringify!($name))).await;
            pavex::middleware::Processing::Continue
        }
    };
}

spy_pre!(first_pre);
spy_pre!(second_pre);
spy_pre!(third_pre);
