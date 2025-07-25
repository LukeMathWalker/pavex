//! Do NOT edit this code.
//! It was automatically generated by Pavex.
//! All manual edits will be lost next time the code is generated.
extern crate alloc;
struct ServerState {
    router: Router,
    #[allow(dead_code)]
    application_state: ApplicationState,
}
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ApplicationConfig {}
pub struct ApplicationState {}
impl ApplicationState {
    pub async fn new(
        _app_config: crate::ApplicationConfig,
    ) -> Result<crate::ApplicationState, crate::ApplicationStateError> {
        Ok(Self::_new().await)
    }
    async fn _new() -> crate::ApplicationState {
        crate::ApplicationState {}
    }
}
#[derive(Debug, thiserror::Error)]
pub enum ApplicationStateError {}
pub fn run(
    server_builder: pavex::server::Server,
    application_state: ApplicationState,
) -> pavex::server::ServerHandle {
    async fn handler(
        request: http::Request<hyper::body::Incoming>,
        connection_info: Option<pavex::connection::ConnectionInfo>,
        server_state: std::sync::Arc<ServerState>,
    ) -> pavex::Response {
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
    domain_router: matchit::Router<u32>,
    domain_0: matchit::Router<u32>,
    domain_1: matchit::Router<u32>,
    domain_2: matchit::Router<u32>,
    domain_3: matchit::Router<u32>,
    domain_4: matchit::Router<u32>,
}
impl Router {
    /// Create a new router instance.
    ///
    /// This method is invoked once, when the server starts.
    pub fn new() -> Self {
        Self {
            domain_router: Self::domain_router(),
            domain_0: Self::domain_0_router(),
            domain_1: Self::domain_1_router(),
            domain_2: Self::domain_2_router(),
            domain_3: Self::domain_3_router(),
            domain_4: Self::domain_4_router(),
        }
    }
    fn domain_router() -> matchit::Router<u32> {
        let mut router = matchit::Router::new();
        router.insert("moc/ynapmoc/nimda", 0u32).unwrap();
        router.insert("moc/ynapmoc", 1u32).unwrap();
        router.insert("moc/ynapmoc/spo", 2u32).unwrap();
        router.insert("moc/ynapmoc/{sub}/{*any}", 3u32).unwrap();
        router.insert("moc/ynapmoc/{sub}", 4u32).unwrap();
        router
    }
    fn domain_0_router() -> matchit::Router<u32> {
        let mut router = matchit::Router::new();
        router.insert("/", 0u32).unwrap();
        router
    }
    fn domain_1_router() -> matchit::Router<u32> {
        let mut router = matchit::Router::new();
        router.insert("/", 0u32).unwrap();
        router.insert("/login", 1u32).unwrap();
        router
    }
    fn domain_2_router() -> matchit::Router<u32> {
        let router = matchit::Router::new();
        router
    }
    fn domain_3_router() -> matchit::Router<u32> {
        let mut router = matchit::Router::new();
        router.insert("/", 0u32).unwrap();
        router
    }
    fn domain_4_router() -> matchit::Router<u32> {
        let mut router = matchit::Router::new();
        router.insert("/", 0u32).unwrap();
        router
    }
    pub async fn route(
        &self,
        request: http::Request<hyper::body::Incoming>,
        connection_info: Option<pavex::connection::ConnectionInfo>,
        state: &ApplicationState,
    ) -> pavex::Response {
        let host: Option<String> = request
            .headers()
            .get(pavex::http::header::HOST)
            .map(|h| pavex::http::uri::Authority::try_from(h.as_bytes()).ok())
            .flatten()
            .map(|a| {
                a.host().trim_end_matches('.').replace('.', "/").chars().rev().collect()
            });
        if let Some(host) = host {
            if let Ok(m) = self.domain_router.at(host.as_str()) {
                return match m.value {
                    0u32 => self.route_domain_0(request, connection_info, state).await,
                    1u32 => self.route_domain_1(request, connection_info, state).await,
                    2u32 => self.route_domain_2(request, connection_info, state).await,
                    3u32 => self.route_domain_3(request, connection_info, state).await,
                    4u32 => self.route_domain_4(request, connection_info, state).await,
                    i => unreachable!("Unknown domain id: {}", i),
                };
            }
        }
        let (request_head, request_body) = request.into_parts();
        #[allow(unused)]
        let request_body = pavex::request::body::RawIncomingBody::from(request_body);
        let request_head: pavex::request::RequestHead = request_head.into();
        route_0::entrypoint(&request_head).await
    }
    async fn route_domain_0(
        &self,
        request: http::Request<hyper::body::Incoming>,
        _connection_info: Option<pavex::connection::ConnectionInfo>,
        #[allow(unused)]
        state: &ApplicationState,
    ) -> pavex::Response {
        let (request_head, _) = request.into_parts();
        let request_head: pavex::request::RequestHead = request_head.into();
        let Ok(matched_route) = self.domain_0.at(&request_head.target.path()) else {
            return route_7::entrypoint().await;
        };
        match matched_route.value {
            0u32 => {
                match &request_head.method {
                    &pavex::http::Method::GET => route_6::entrypoint().await,
                    _ => route_7::entrypoint().await,
                }
            }
            i => unreachable!("Unknown route id: {}", i),
        }
    }
    async fn route_domain_1(
        &self,
        request: http::Request<hyper::body::Incoming>,
        _connection_info: Option<pavex::connection::ConnectionInfo>,
        #[allow(unused)]
        state: &ApplicationState,
    ) -> pavex::Response {
        let (request_head, _) = request.into_parts();
        let request_head: pavex::request::RequestHead = request_head.into();
        let Ok(matched_route) = self.domain_1.at(&request_head.target.path()) else {
            return route_0::entrypoint(&request_head).await;
        };
        match matched_route.value {
            0u32 => {
                match &request_head.method {
                    &pavex::http::Method::GET => route_4::entrypoint().await,
                    _ => route_0::entrypoint(&request_head).await,
                }
            }
            1u32 => {
                match &request_head.method {
                    &pavex::http::Method::GET => route_5::entrypoint().await,
                    _ => route_0::entrypoint(&request_head).await,
                }
            }
            i => unreachable!("Unknown route id: {}", i),
        }
    }
    async fn route_domain_2(
        &self,
        request: http::Request<hyper::body::Incoming>,
        _connection_info: Option<pavex::connection::ConnectionInfo>,
        #[allow(unused)]
        state: &ApplicationState,
    ) -> pavex::Response {
        let (request_head, _) = request.into_parts();
        let request_head: pavex::request::RequestHead = request_head.into();
        let Ok(matched_route) = self.domain_2.at(&request_head.target.path()) else {
            return route_3::entrypoint().await;
        };
        match matched_route.value {
            i => unreachable!("Unknown route id: {}", i),
        }
    }
    async fn route_domain_3(
        &self,
        request: http::Request<hyper::body::Incoming>,
        _connection_info: Option<pavex::connection::ConnectionInfo>,
        #[allow(unused)]
        state: &ApplicationState,
    ) -> pavex::Response {
        let (request_head, _) = request.into_parts();
        let request_head: pavex::request::RequestHead = request_head.into();
        let Ok(matched_route) = self.domain_3.at(&request_head.target.path()) else {
            return route_0::entrypoint(&request_head).await;
        };
        match matched_route.value {
            0u32 => {
                match &request_head.method {
                    &pavex::http::Method::GET => route_1::entrypoint().await,
                    _ => route_0::entrypoint(&request_head).await,
                }
            }
            i => unreachable!("Unknown route id: {}", i),
        }
    }
    async fn route_domain_4(
        &self,
        request: http::Request<hyper::body::Incoming>,
        _connection_info: Option<pavex::connection::ConnectionInfo>,
        #[allow(unused)]
        state: &ApplicationState,
    ) -> pavex::Response {
        let (request_head, _) = request.into_parts();
        let request_head: pavex::request::RequestHead = request_head.into();
        let Ok(matched_route) = self.domain_4.at(&request_head.target.path()) else {
            return route_0::entrypoint(&request_head).await;
        };
        match matched_route.value {
            0u32 => {
                match &request_head.method {
                    &pavex::http::Method::GET => route_2::entrypoint().await,
                    _ => route_0::entrypoint(&request_head).await,
                }
            }
            i => unreachable!("Unknown route id: {}", i),
        }
    }
}
pub mod route_0 {
    pub async fn entrypoint<'a>(
        s_0: &'a pavex::request::RequestHead,
    ) -> pavex::Response {
        let response = wrapping_0(s_0).await;
        response
    }
    async fn stage_1<'a>(s_0: &'a pavex::request::RequestHead) -> pavex::Response {
        let response = handler(s_0).await;
        response
    }
    async fn wrapping_0(v0: &pavex::request::RequestHead) -> pavex::Response {
        let v1 = crate::route_0::Next0 {
            s_0: v0,
            next: stage_1,
        };
        let v2 = pavex::middleware::Next::new(v1);
        let v3 = pavex::middleware::wrap_noop(v2).await;
        <pavex::Response as pavex::IntoResponse>::into_response(v3)
    }
    async fn handler(v0: &pavex::request::RequestHead) -> pavex::Response {
        let v1 = app::root_fallback(v0);
        <pavex::Response as pavex::IntoResponse>::into_response(v1)
    }
    struct Next0<'a, T>
    where
        T: std::future::Future<Output = pavex::Response>,
    {
        s_0: &'a pavex::request::RequestHead,
        next: fn(&'a pavex::request::RequestHead) -> T,
    }
    impl<'a, T> std::future::IntoFuture for Next0<'a, T>
    where
        T: std::future::Future<Output = pavex::Response>,
    {
        type Output = pavex::Response;
        type IntoFuture = T;
        fn into_future(self) -> Self::IntoFuture {
            (self.next)(self.s_0)
        }
    }
}
pub mod route_1 {
    pub async fn entrypoint() -> pavex::Response {
        let response = wrapping_0().await;
        response
    }
    async fn stage_1() -> pavex::Response {
        let response = handler().await;
        response
    }
    async fn wrapping_0() -> pavex::Response {
        let v0 = crate::route_1::Next0 {
            next: stage_1,
        };
        let v1 = pavex::middleware::Next::new(v0);
        let v2 = pavex::middleware::wrap_noop(v1).await;
        <pavex::Response as pavex::IntoResponse>::into_response(v2)
    }
    async fn handler() -> pavex::Response {
        let v0 = app::base_any();
        <pavex::Response as pavex::IntoResponse>::into_response(v0)
    }
    struct Next0<T>
    where
        T: std::future::Future<Output = pavex::Response>,
    {
        next: fn() -> T,
    }
    impl<T> std::future::IntoFuture for Next0<T>
    where
        T: std::future::Future<Output = pavex::Response>,
    {
        type Output = pavex::Response;
        type IntoFuture = T;
        fn into_future(self) -> Self::IntoFuture {
            (self.next)()
        }
    }
}
pub mod route_2 {
    pub async fn entrypoint() -> pavex::Response {
        let response = wrapping_0().await;
        response
    }
    async fn stage_1() -> pavex::Response {
        let response = handler().await;
        response
    }
    async fn wrapping_0() -> pavex::Response {
        let v0 = crate::route_2::Next0 {
            next: stage_1,
        };
        let v1 = pavex::middleware::Next::new(v0);
        let v2 = pavex::middleware::wrap_noop(v1).await;
        <pavex::Response as pavex::IntoResponse>::into_response(v2)
    }
    async fn handler() -> pavex::Response {
        let v0 = app::base_sub();
        <pavex::Response as pavex::IntoResponse>::into_response(v0)
    }
    struct Next0<T>
    where
        T: std::future::Future<Output = pavex::Response>,
    {
        next: fn() -> T,
    }
    impl<T> std::future::IntoFuture for Next0<T>
    where
        T: std::future::Future<Output = pavex::Response>,
    {
        type Output = pavex::Response;
        type IntoFuture = T;
        fn into_future(self) -> Self::IntoFuture {
            (self.next)()
        }
    }
}
pub mod route_3 {
    pub async fn entrypoint() -> pavex::Response {
        let response = wrapping_0().await;
        response
    }
    async fn stage_1() -> pavex::Response {
        let response = handler().await;
        response
    }
    async fn wrapping_0() -> pavex::Response {
        let v0 = crate::route_3::Next0 {
            next: stage_1,
        };
        let v1 = pavex::middleware::Next::new(v0);
        let v2 = pavex::middleware::wrap_noop(v1).await;
        <pavex::Response as pavex::IntoResponse>::into_response(v2)
    }
    async fn handler() -> pavex::Response {
        let v0 = app::ops_fallback();
        <pavex::Response as pavex::IntoResponse>::into_response(v0)
    }
    struct Next0<T>
    where
        T: std::future::Future<Output = pavex::Response>,
    {
        next: fn() -> T,
    }
    impl<T> std::future::IntoFuture for Next0<T>
    where
        T: std::future::Future<Output = pavex::Response>,
    {
        type Output = pavex::Response;
        type IntoFuture = T;
        fn into_future(self) -> Self::IntoFuture {
            (self.next)()
        }
    }
}
pub mod route_4 {
    pub async fn entrypoint() -> pavex::Response {
        let response = wrapping_0().await;
        response
    }
    async fn stage_1() -> pavex::Response {
        let response = handler().await;
        response
    }
    async fn wrapping_0() -> pavex::Response {
        let v0 = crate::route_4::Next0 {
            next: stage_1,
        };
        let v1 = pavex::middleware::Next::new(v0);
        let v2 = pavex::middleware::wrap_noop(v1).await;
        <pavex::Response as pavex::IntoResponse>::into_response(v2)
    }
    async fn handler() -> pavex::Response {
        let v0 = app::base_root();
        <pavex::Response as pavex::IntoResponse>::into_response(v0)
    }
    struct Next0<T>
    where
        T: std::future::Future<Output = pavex::Response>,
    {
        next: fn() -> T,
    }
    impl<T> std::future::IntoFuture for Next0<T>
    where
        T: std::future::Future<Output = pavex::Response>,
    {
        type Output = pavex::Response;
        type IntoFuture = T;
        fn into_future(self) -> Self::IntoFuture {
            (self.next)()
        }
    }
}
pub mod route_5 {
    pub async fn entrypoint() -> pavex::Response {
        let response = wrapping_0().await;
        response
    }
    async fn stage_1() -> pavex::Response {
        let response = handler().await;
        response
    }
    async fn wrapping_0() -> pavex::Response {
        let v0 = crate::route_5::Next0 {
            next: stage_1,
        };
        let v1 = pavex::middleware::Next::new(v0);
        let v2 = pavex::middleware::wrap_noop(v1).await;
        <pavex::Response as pavex::IntoResponse>::into_response(v2)
    }
    async fn handler() -> pavex::Response {
        let v0 = app::base_login();
        <pavex::Response as pavex::IntoResponse>::into_response(v0)
    }
    struct Next0<T>
    where
        T: std::future::Future<Output = pavex::Response>,
    {
        next: fn() -> T,
    }
    impl<T> std::future::IntoFuture for Next0<T>
    where
        T: std::future::Future<Output = pavex::Response>,
    {
        type Output = pavex::Response;
        type IntoFuture = T;
        fn into_future(self) -> Self::IntoFuture {
            (self.next)()
        }
    }
}
pub mod route_6 {
    pub async fn entrypoint() -> pavex::Response {
        let response = wrapping_0().await;
        response
    }
    async fn stage_1() -> pavex::Response {
        let response = handler().await;
        response
    }
    async fn wrapping_0() -> pavex::Response {
        let v0 = crate::route_6::Next0 {
            next: stage_1,
        };
        let v1 = pavex::middleware::Next::new(v0);
        let v2 = pavex::middleware::wrap_noop(v1).await;
        <pavex::Response as pavex::IntoResponse>::into_response(v2)
    }
    async fn handler() -> pavex::Response {
        let v0 = app::admin_root();
        <pavex::Response as pavex::IntoResponse>::into_response(v0)
    }
    struct Next0<T>
    where
        T: std::future::Future<Output = pavex::Response>,
    {
        next: fn() -> T,
    }
    impl<T> std::future::IntoFuture for Next0<T>
    where
        T: std::future::Future<Output = pavex::Response>,
    {
        type Output = pavex::Response;
        type IntoFuture = T;
        fn into_future(self) -> Self::IntoFuture {
            (self.next)()
        }
    }
}
pub mod route_7 {
    pub async fn entrypoint() -> pavex::Response {
        let response = wrapping_0().await;
        response
    }
    async fn stage_1() -> pavex::Response {
        let response = handler().await;
        response
    }
    async fn wrapping_0() -> pavex::Response {
        let v0 = crate::route_7::Next0 {
            next: stage_1,
        };
        let v1 = pavex::middleware::Next::new(v0);
        let v2 = pavex::middleware::wrap_noop(v1).await;
        <pavex::Response as pavex::IntoResponse>::into_response(v2)
    }
    async fn handler() -> pavex::Response {
        let v0 = app::admin_fallback();
        <pavex::Response as pavex::IntoResponse>::into_response(v0)
    }
    struct Next0<T>
    where
        T: std::future::Future<Output = pavex::Response>,
    {
        next: fn() -> T,
    }
    impl<T> std::future::IntoFuture for Next0<T>
    where
        T: std::future::Future<Output = pavex::Response>,
    {
        type Output = pavex::Response;
        type IntoFuture = T;
        fn into_future(self) -> Self::IntoFuture {
            (self.next)()
        }
    }
}