//! Do NOT edit this code.
//! It was automatically generated by Pavex.
//! All manual edits will be lost next time the code is generated.
extern crate alloc;
struct ServerState {
    router: pavex_matchit::Router<u32>,
    #[allow(dead_code)]
    application_state: ApplicationState,
}
pub struct ApplicationState {}
pub async fn build_application_state() -> crate::ApplicationState {
    crate::ApplicationState {}
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
fn build_router() -> pavex_matchit::Router<u32> {
    let mut router = pavex_matchit::Router::new();
    router.insert("/head", 0u32).unwrap();
    router.insert("/parts", 1u32).unwrap();
    router.insert("/response", 2u32).unwrap();
    router.insert("/status_code", 3u32).unwrap();
    router
}
async fn route_request(
    request: http::Request<hyper::body::Incoming>,
    server_state: std::sync::Arc<ServerState>,
) -> pavex::response::Response {
    let (request_head, request_body) = request.into_parts();
    #[allow(unused)]
    let request_body = pavex::request::body::RawIncomingBody::from(request_body);
    let request_head: pavex::request::RequestHead = request_head.into();
    let matched_route = match server_state.router.at(&request_head.target.path()) {
        Ok(m) => m,
        Err(_) => {
            let allowed_methods: pavex::router::AllowedMethods = pavex::router::MethodAllowList::from_iter(
                    vec![],
                )
                .into();
            return route_4::handler(&allowed_methods).await;
        }
    };
    let route_id = matched_route.value;
    #[allow(unused)]
    let url_params: pavex::request::path::RawPathParams<'_, '_> = matched_route
        .params
        .into();
    match route_id {
        0u32 => {
            match &request_head.method {
                &pavex::http::Method::GET => route_3::handler().await,
                _ => {
                    let allowed_methods: pavex::router::AllowedMethods = pavex::router::MethodAllowList::from_iter([
                            pavex::http::Method::GET,
                        ])
                        .into();
                    route_4::handler(&allowed_methods).await
                }
            }
        }
        1u32 => {
            match &request_head.method {
                &pavex::http::Method::GET => route_2::handler().await,
                _ => {
                    let allowed_methods: pavex::router::AllowedMethods = pavex::router::MethodAllowList::from_iter([
                            pavex::http::Method::GET,
                        ])
                        .into();
                    route_4::handler(&allowed_methods).await
                }
            }
        }
        2u32 => {
            match &request_head.method {
                &pavex::http::Method::GET => route_0::handler().await,
                _ => {
                    let allowed_methods: pavex::router::AllowedMethods = pavex::router::MethodAllowList::from_iter([
                            pavex::http::Method::GET,
                        ])
                        .into();
                    route_4::handler(&allowed_methods).await
                }
            }
        }
        3u32 => {
            match &request_head.method {
                &pavex::http::Method::GET => route_1::handler().await,
                _ => {
                    let allowed_methods: pavex::router::AllowedMethods = pavex::router::MethodAllowList::from_iter([
                            pavex::http::Method::GET,
                        ])
                        .into();
                    route_4::handler(&allowed_methods).await
                }
            }
        }
        i => unreachable!("Unknown route id: {}", i),
    }
}
pub mod route_0 {
    pub async fn handler() -> pavex::response::Response {
        let v0 = app::response();
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v0)
    }
}
pub mod route_1 {
    pub async fn handler() -> pavex::response::Response {
        let v0 = app::status_code();
        <http::StatusCode as pavex::response::IntoResponse>::into_response(v0)
    }
}
pub mod route_2 {
    pub async fn handler() -> pavex::response::Response {
        let v0 = app::parts();
        <http::response::Parts as pavex::response::IntoResponse>::into_response(v0)
    }
}
pub mod route_3 {
    pub async fn handler() -> pavex::response::Response {
        let v0 = app::response_head();
        <pavex::response::ResponseHead as pavex::response::IntoResponse>::into_response(
            v0,
        )
    }
}
pub mod route_4 {
    pub async fn handler(
        v0: &pavex::router::AllowedMethods,
    ) -> pavex::response::Response {
        let v1 = pavex::router::default_fallback(v0).await;
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v1)
    }
}