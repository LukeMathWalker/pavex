//! Do NOT edit this code.
//! It was automatically generated by Pavex.
//! All manual edits will be lost next time the code is generated.
#[allow(unused_imports)]
use std as alloc;
struct ServerState {
    router: pavex::routing::Router<u32>,
    application_state: ApplicationState,
}
pub struct ApplicationState {
    s0: app::HttpClient,
}
#[derive(Debug, thiserror::Error)]
pub enum ApplicationStateError {
    #[error(transparent)]
    HttpClient(app::HttpClientError),
}
pub async fn build_application_state(
    v0: app::Config,
) -> Result<crate::ApplicationState, crate::ApplicationStateError> {
    let v1 = app::http_client(v0);
    let v2 = match v1 {
        Ok(ok) => ok,
        Err(v2) => {
            return {
                let v3 = crate::ApplicationStateError::HttpClient(v2);
                core::result::Result::Err(v3)
            };
        }
    };
    let v3 = crate::ApplicationState { s0: v2 };
    core::result::Result::Ok(v3)
}
pub fn run(
    server_builder: pavex::server::Server,
    application_state: ApplicationState,
) -> pavex::server::ServerHandle {
    let server_state = std::sync::Arc::new(ServerState {
        router: build_router(),
        application_state,
    });
    server_builder.serve(route_request, server_state)
}
fn build_router() -> pavex::routing::Router<u32> {
    let mut router = pavex::routing::Router::new();
    router.insert("/home", 0u32).unwrap();
    router
}
async fn route_request(
    request: http::Request<pavex::hyper::body::Incoming>,
    server_state: std::sync::Arc<ServerState>,
) -> pavex::response::Response {
    #[allow(unused)]
    let (request_head, request_body) = request.into_parts();
    let request_head: pavex::request::RequestHead = request_head.into();
    let matched_route = match server_state.router.at(&request_head.uri.path()) {
        Ok(m) => m,
        Err(_) => {
            let allowed_methods = pavex::extract::route::AllowedMethods::new(vec![]);
            return route_1::middleware_0(&allowed_methods).await;
        }
    };
    let route_id = matched_route.value;
    #[allow(unused)]
    let url_params: pavex::extract::route::RawRouteParams<'_, '_> = matched_route
        .params
        .into();
    match route_id {
        0u32 => {
            match &request_head.method {
                &pavex::http::Method::GET => {
                    route_0::middleware_0(
                            request_head,
                            server_state.application_state.s0.clone(),
                        )
                        .await
                }
                _ => {
                    let allowed_methods = pavex::extract::route::AllowedMethods::new(
                        vec![pavex::http::Method::GET],
                    );
                    route_1::middleware_0(&allowed_methods).await
                }
            }
        }
        _ => pavex::response::Response::not_found().box_body(),
    }
}
pub mod route_0 {
    pub async fn middleware_0(
        v0: pavex::request::RequestHead,
        v1: app::HttpClient,
    ) -> pavex::response::Response {
        let v2 = crate::route_0::Next0 {
            s_0: v1,
            s_1: v0,
            next: handler,
        };
        let v3 = pavex::middleware::Next::new(v2);
        let v4 = app::fallible_wrapping_middleware(v3);
        let v5 = match v4 {
            Ok(ok) => ok,
            Err(v5) => {
                return {
                    let v6 = app::handle_middleware_error(&v5);
                    <pavex::response::Response as pavex::response::IntoResponse>::into_response(
                        v6,
                    )
                };
            }
        };
        v5
    }
    pub async fn handler(
        v0: app::HttpClient,
        v1: pavex::request::RequestHead,
    ) -> pavex::response::Response {
        let v2 = match app::logger() {
            Ok(ok) => ok,
            Err(v2) => {
                return {
                    let v3 = app::handle_logger_error(&v2);
                    <pavex::response::Response as pavex::response::IntoResponse>::into_response(
                        v3,
                    )
                };
            }
        };
        let v3 = app::extract_path(v1);
        let v4 = match v3 {
            Ok(ok) => ok,
            Err(v4) => {
                return {
                    let v5 = match app::logger() {
                        Ok(ok) => ok,
                        Err(v5) => {
                            return {
                                let v6 = app::handle_logger_error(&v5);
                                <pavex::response::Response as pavex::response::IntoResponse>::into_response(
                                    v6,
                                )
                            };
                        }
                    };
                    let v6 = app::handle_extract_path_error(&v4, v5);
                    <pavex::response::Response as pavex::response::IntoResponse>::into_response(
                        v6,
                    )
                };
            }
        };
        let v5 = app::request_handler(v4, v2, v0);
        let v6 = match v5 {
            Ok(ok) => ok,
            Err(v6) => {
                return {
                    let v7 = app::handle_handler_error(&v6);
                    <pavex::response::Response as pavex::response::IntoResponse>::into_response(
                        v7,
                    )
                };
            }
        };
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v6)
    }
    pub struct Next0<T>
    where
        T: std::future::Future<Output = pavex::response::Response>,
    {
        s_0: app::HttpClient,
        s_1: pavex::request::RequestHead,
        next: fn(app::HttpClient, pavex::request::RequestHead) -> T,
    }
    impl<T> std::future::IntoFuture for Next0<T>
    where
        T: std::future::Future<Output = pavex::response::Response>,
    {
        type Output = pavex::response::Response;
        type IntoFuture = T;
        fn into_future(self) -> Self::IntoFuture {
            (self.next)(self.s_0, self.s_1)
        }
    }
}
pub mod route_1 {
    pub async fn middleware_0(
        v0: &pavex::extract::route::AllowedMethods,
    ) -> pavex::response::Response {
        let v1 = crate::route_1::Next0 {
            s_0: v0,
            next: handler,
        };
        let v2 = pavex::middleware::Next::new(v1);
        let v3 = app::fallible_wrapping_middleware(v2);
        let v4 = match v3 {
            Ok(ok) => ok,
            Err(v4) => {
                return {
                    let v5 = app::handle_middleware_error(&v4);
                    <pavex::response::Response as pavex::response::IntoResponse>::into_response(
                        v5,
                    )
                };
            }
        };
        v4
    }
    pub async fn handler(
        v0: &pavex::extract::route::AllowedMethods,
    ) -> pavex::response::Response {
        let v1 = pavex::router::default_fallback(v0).await;
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v1)
    }
    pub struct Next0<'a, T>
    where
        T: std::future::Future<Output = pavex::response::Response>,
    {
        s_0: &'a pavex::extract::route::AllowedMethods,
        next: fn(&'a pavex::extract::route::AllowedMethods) -> T,
    }
    impl<'a, T> std::future::IntoFuture for Next0<'a, T>
    where
        T: std::future::Future<Output = pavex::response::Response>,
    {
        type Output = pavex::response::Response;
        type IntoFuture = T;
        fn into_future(self) -> Self::IntoFuture {
            (self.next)(self.s_0)
        }
    }
}