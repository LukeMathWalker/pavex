//! Do NOT edit this code.
//! It was automatically generated by Pavex.
//! All manual edits will be lost next time the code is generated.
extern crate alloc;
struct ServerState {
    router: Router,
    application_state: ApplicationState,
}
pub struct ApplicationState {
    http_client: app::HttpClient,
}
pub async fn build_application_state(
    v0: app::Config,
) -> crate::ApplicationState {
    let v1 = app::http_client(v0);
    crate::ApplicationState {
        http_client: v1,
    }
}
pub fn run(
    server_builder: pavex::server::Server,
    application_state: ApplicationState,
) -> pavex::server::ServerHandle {
    async fn handler(
        request: http::Request<hyper::body::Incoming>,
        connection_info: Option<pavex::connection::ConnectionInfo>,
        server_state: std::sync::Arc<ServerState>,
    ) -> pavex::response::Response {
        let (router, state) = (&server_state.router, &server_state.application_state);
        router.route(request, connection_info, state).await
    }
    let router = Router::new();
    let server_state = std::sync::Arc::new(ServerState {
        router,
        application_state,
    });
    server_builder.serve(handler, server_state)
}
struct Router {
    router: matchit::Router<u32>,
}
impl Router {
    /// Create a new router instance.
    ///
    /// This method is invoked once, when the server starts.
    pub fn new() -> Self {
        Self { router: Self::router() }
    }
    fn router() -> matchit::Router<u32> {
        let mut router = matchit::Router::new();
        router.insert("/home", 0u32).unwrap();
        router
    }
    pub async fn route(
        &self,
        request: http::Request<hyper::body::Incoming>,
        _connection_info: Option<pavex::connection::ConnectionInfo>,
        #[allow(unused)]
        state: &ApplicationState,
    ) -> pavex::response::Response {
        let (request_head, _) = request.into_parts();
        let request_head: pavex::request::RequestHead = request_head.into();
        let Ok(matched_route) = self.router.at(&request_head.target.path()) else {
            let allowed_methods: pavex::router::AllowedMethods = pavex::router::MethodAllowList::from_iter(
                    vec![],
                )
                .into();
            return route_1::entrypoint(&allowed_methods).await;
        };
        match matched_route.value {
            0u32 => {
                match &request_head.method {
                    &pavex::http::Method::GET => {
                        route_0::entrypoint(state.http_client.clone(), request_head)
                            .await
                    }
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
}
pub mod route_0 {
    pub async fn entrypoint(
        s_0: app::HttpClient,
        s_1: pavex::request::RequestHead,
    ) -> pavex::response::Response {
        let response = wrapping_0(s_0, s_1).await;
        response
    }
    async fn stage_1(
        s_0: app::HttpClient,
        s_1: pavex::request::RequestHead,
    ) -> pavex::response::Response {
        let response = handler(s_0, s_1).await;
        response
    }
    async fn wrapping_0(
        v0: app::HttpClient,
        v1: pavex::request::RequestHead,
    ) -> pavex::response::Response {
        let v2 = crate::route_0::Next0 {
            s_0: v0,
            s_1: v1,
            next: stage_1,
        };
        let v3 = pavex::middleware::Next::new(v2);
        let v4 = pavex::middleware::wrap_noop(v3).await;
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v4)
    }
    async fn handler(
        v0: app::HttpClient,
        v1: pavex::request::RequestHead,
    ) -> pavex::response::Response {
        let v2 = app::extract_path(v1).await;
        let v3 = match v2 {
            Ok(ok) => ok,
            Err(v3) => {
                return {
                    let v4 = app::logger();
                    let v5 = app::handle_extract_path_error(&v3, v4);
                    <pavex::response::Response as pavex::response::IntoResponse>::into_response(
                        v5,
                    )
                };
            }
        };
        let v4 = app::logger();
        let v5 = app::stream_file(v3, v4, v0);
        <pavex::response::Response as pavex::response::IntoResponse>::into_response(v5)
    }
    struct Next0<T>
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
    pub async fn entrypoint<'a>(
        s_0: &'a pavex::router::AllowedMethods,
    ) -> pavex::response::Response {
        let response = wrapping_0(s_0).await;
        response
    }
    async fn stage_1<'a>(
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
}