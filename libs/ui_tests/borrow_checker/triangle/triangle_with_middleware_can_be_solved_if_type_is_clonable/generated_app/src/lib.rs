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
    router.insert("/home", 0u32).unwrap();
    router
}
async fn route_request(
    request: http::Request<hyper::body::Incoming>,
    _connection_info: Option<pavex::connection::ConnectionInfo>,
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
            return route_1::entrypoint(&allowed_methods).await;
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
                &pavex::http::Method::GET => route_0::entrypoint().await,
                _ => {
                    let allowed_methods: pavex::router::AllowedMethods = pavex::router::MethodAllowList::from_iter([
                            pavex::http::Method::GET,
                        ])
                        .into();
                    route_1::entrypoint(&allowed_methods).await
                }
            }
        }
        i => unreachable!("Unknown route id: {}", i),
    }
}
pub mod route_0 {
    pub async fn entrypoint() -> pavex::response::Response {
        let response = wrapping_0().await;
        response
    }
    async fn stage_1() -> pavex::response::Response {
        let response = wrapping_1().await;
        response
    }
    async fn stage_2<'a>(s_0: &'a app_01492b3d::A) -> pavex::response::Response {
        let response = handler(s_0).await;
        response
    }
    async fn wrapping_0() -> pavex::response::Response {
        let v0 = crate::route_0::Next0 {
            next: stage_1,
        };
        let v1 = pavex::middleware::Next::new(v0);
        let v2 = pavex::middleware::wrap_noop(v1).await;
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v2)
    }
    async fn wrapping_1() -> pavex::response::Response {
        let v0 = app_01492b3d::a();
        let v1 = crate::route_0::Next1 {
            s_0: &v0,
            next: stage_2,
        };
        let v2 = pavex::middleware::Next::new(v1);
        let v3 = <app_01492b3d::A as core::clone::Clone>::clone(&v0);
        let v4 = app_01492b3d::mw(v3, v2);
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v4)
    }
    async fn handler(v0: &app_01492b3d::A) -> pavex::response::Response {
        let v1 = app_01492b3d::handler(v0);
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v1)
    }
    struct Next0<T>
    where
        T: std::future::Future<Output = pavex::response::Response>,
    {
        next: fn() -> T,
    }
    impl<T> std::future::IntoFuture for Next0<T>
    where
        T: std::future::Future<Output = pavex::response::Response>,
    {
        type Output = pavex::response::Response;
        type IntoFuture = T;
        fn into_future(self) -> Self::IntoFuture {
            (self.next)()
        }
    }
    struct Next1<'a, T>
    where
        T: std::future::Future<Output = pavex::response::Response>,
    {
        s_0: &'a app_01492b3d::A,
        next: fn(&'a app_01492b3d::A) -> T,
    }
    impl<'a, T> std::future::IntoFuture for Next1<'a, T>
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
pub mod route_1 {
    pub async fn entrypoint<'a>(
        s_0: &'a pavex::router::AllowedMethods,
    ) -> pavex::response::Response {
        let response = wrapping_0(s_0).await;
        response
    }
    async fn stage_1<'a>(
        s_0: &'a pavex::router::AllowedMethods,
    ) -> pavex::response::Response {
        let response = wrapping_1(s_0).await;
        response
    }
    async fn stage_2<'a>(
        s_0: &'a pavex::router::AllowedMethods,
    ) -> pavex::response::Response {
        let response = handler(s_0).await;
        response
    }
    async fn wrapping_0(
        v0: &pavex::router::AllowedMethods,
    ) -> pavex::response::Response {
        let v1 = crate::route_1::Next0 {
            s_0: v0,
            next: stage_1,
        };
        let v2 = pavex::middleware::Next::new(v1);
        let v3 = pavex::middleware::wrap_noop(v2).await;
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v3)
    }
    async fn wrapping_1(
        v0: &pavex::router::AllowedMethods,
    ) -> pavex::response::Response {
        let v1 = crate::route_1::Next1 {
            s_0: v0,
            next: stage_2,
        };
        let v2 = pavex::middleware::Next::new(v1);
        let v3 = app_01492b3d::a();
        let v4 = app_01492b3d::mw(v3, v2);
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v4)
    }
    async fn handler(v0: &pavex::router::AllowedMethods) -> pavex::response::Response {
        let v1 = pavex::router::default_fallback(v0).await;
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v1)
    }
    struct Next0<'a, T>
    where
        T: std::future::Future<Output = pavex::response::Response>,
    {
        s_0: &'a pavex::router::AllowedMethods,
        next: fn(&'a pavex::router::AllowedMethods) -> T,
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
    struct Next1<'a, T>
    where
        T: std::future::Future<Output = pavex::response::Response>,
    {
        s_0: &'a pavex::router::AllowedMethods,
        next: fn(&'a pavex::router::AllowedMethods) -> T,
    }
    impl<'a, T> std::future::IntoFuture for Next1<'a, T>
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
