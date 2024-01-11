use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::{request::path::PathParams, response::Response};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub struct SpyState(Arc<Mutex<Vec<String>>>);

impl SpyState {
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

#[derive(Clone, Debug)]
pub struct Spy {
    state: SpyState,
}

impl Spy {
    pub fn new(state: SpyState) -> Self {
        Self { state }
    }

    pub async fn push(&self, s: String) {
        self.state.push(s).await
    }

    pub async fn get(&self) -> Vec<String> {
        self.state.get().await
    }
}

pub async fn handler(spy: &Spy) -> Response {
    spy.push("handler".to_string()).await;
    Response::ok()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::Spy::new), Lifecycle::Singleton);
    bp.wrap(f!(crate::first));
    bp.wrap(f!(crate::second));
    bp.route(GET, "/", f!(crate::handler));
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
