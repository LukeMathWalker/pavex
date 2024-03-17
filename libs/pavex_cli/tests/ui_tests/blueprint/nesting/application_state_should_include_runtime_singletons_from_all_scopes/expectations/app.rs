//! Do NOT edit this code.
//! It was automatically generated by Pavex.
//! All manual edits will be lost next time the code is generated.
extern crate alloc;
struct ServerState {
    router: pavex_matchit::Router<u32>,
    application_state: ApplicationState,
}
pub struct ApplicationState {
    s0: u64,
    s1: u32,
}
pub async fn build_application_state() -> crate::ApplicationState {
    let v0 = app::singleton_dep();
    let v1 = app::nested_singleton(v0);
    let v2 = app::parent_singleton();
    crate::ApplicationState {
        s0: v2,
        s1: v1,
    }
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
    router.insert("/child", 0u32).unwrap();
    router.insert("/parent", 1u32).unwrap();
    router
}
async fn route_request(
    request: http::Request<hyper::body::Incoming>,
    connection_info: Option<pavex::connection::ConnectionInfo>,
    server_state: std::sync::Arc<ServerState>,
) -> pavex::response::Response {
    let (request_head, request_body) = request.into_parts();
    let _ = connection_info;
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
            return route_1::handler(&allowed_methods).await;
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
                &pavex::http::Method::GET => {
                    route_2::handler(server_state.application_state.s1.clone()).await
                }
                _ => {
                    let allowed_methods: pavex::router::AllowedMethods = pavex::router::MethodAllowList::from_iter([
                            pavex::http::Method::GET,
                        ])
                        .into();
                    route_1::handler(&allowed_methods).await
                }
            }
        }
        1u32 => {
            match &request_head.method {
                &pavex::http::Method::GET => {
                    route_0::handler(server_state.application_state.s0.clone()).await
                }
                _ => {
                    let allowed_methods: pavex::router::AllowedMethods = pavex::router::MethodAllowList::from_iter([
                            pavex::http::Method::GET,
                        ])
                        .into();
                    route_1::handler(&allowed_methods).await
                }
            }
        }
        i => unreachable!("Unknown route id: {}", i),
    }
}
pub mod route_0 {
    pub async fn handler(v0: u64) -> pavex::response::Response {
        let v1 = app::parent_handler(v0);
        <http::StatusCode as pavex::response::IntoResponse>::into_response(v1)
    }
}
pub mod route_1 {
    pub async fn handler(
        v0: &pavex::router::AllowedMethods,
    ) -> pavex::response::Response {
        let v1 = pavex::router::default_fallback(v0).await;
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v1)
    }
}
pub mod route_2 {
    pub async fn handler(v0: u32) -> pavex::response::Response {
        let v1 = app::nested_handler(v0);
        <http::StatusCode as pavex::response::IntoResponse>::into_response(v1)
    }
}