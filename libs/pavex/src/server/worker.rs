use std::future::Future;
use std::thread;

use anyhow::Context;
use tokio::net::TcpStream;
use tokio::sync::mpsc::error::TrySendError;

/// A handle to dispatch incoming connections to a worker thread.
pub(super) struct WorkerHandle {
    connection_outbox: tokio::sync::mpsc::Sender<TcpStream>,
    id: usize,
}

impl WorkerHandle {
    /// Dispatch a connection to the worker thread.
    pub(super) fn dispatch(&self, connection: TcpStream) -> Result<(), TrySendError<TcpStream>> {
        self.connection_outbox.try_send(connection)
    }

    /// Get the worker's ID.
    pub(super) fn id(&self) -> usize {
        self.id
    }
}

#[must_use]
/// A worker thread that handles incoming connections.
pub(super) struct Worker<HandlerFuture, ApplicationState> {
    connection_inbox: tokio::sync::mpsc::Receiver<TcpStream>,
    handler: fn(http::Request<hyper::body::Incoming>, ApplicationState) -> HandlerFuture,
    application_state: ApplicationState,
    id: usize,
}

impl<HandlerFuture, ApplicationState> Worker<HandlerFuture, ApplicationState>
where
    HandlerFuture: Future<Output = crate::response::Response> + 'static,
    ApplicationState: Clone + Send + Sync + 'static,
{
    /// Configure a new worker without spawning it.
    ///
    /// `max_queue_length` is the maximum number of connections that can be queued up for this
    /// worker.
    pub(super) fn new(
        id: usize,
        max_queue_length: usize,
        handler: fn(http::Request<hyper::body::Incoming>, ApplicationState) -> HandlerFuture,
        application_state: ApplicationState,
    ) -> (Self, WorkerHandle) {
        let (connection_outbox, connection_inbox) = tokio::sync::mpsc::channel(max_queue_length);
        let self_ = Self {
            connection_inbox,
            handler,
            application_state,
            id,
        };
        let handle = WorkerHandle {
            connection_outbox,
            id,
        };
        (self_, handle)
    }

    /// Run the worker: wait for incoming connections and handle them.
    async fn run(mut self) {
        // TODO: expose all the config options for `Http` through the top-level `ServerConfiguration`
        // object.
        let connection_handler = hyper_util::server::conn::auto::Builder::new(LocalExec);
        while let Some(connection) = self.connection_inbox.recv().await {
            let handler = hyper::service::service_fn(|request| {
                let handler = (self.handler)(request, self.application_state.clone());
                async move {
                    let response = handler.await;
                    let response = crate::hyper::Response::from(response);
                    Ok::<_, hyper::Error>(response)
                }
            });
            connection_handler
                .serve_connection(connection, handler)
                .await
                .unwrap();
            println!("Worker {} received a connection", self.id);
        }
        tracing::info!("Worker {} finished", self.id);
    }

    /// Spawn a thread and run the worker there, using a single-threaded executor that can
    /// handle !Send futures.
    pub(super) fn spawn(self) -> Result<thread::JoinHandle<()>, anyhow::Error> {
        thread::Builder::new()
            .name(format!("pavex-worker-{}", self.id))
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to build single-threaded Tokio runtime for worker thread");

                // Use a `LocalSet` to allow the worker to spawn !Send futures.
                let local = tokio::task::LocalSet::new();
                local.block_on(&runtime, self.run());
            })
            .context("Failed to spawn worker thread")
    }
}

/// HTTP2 requires `hyper` to be able to spawn tasks, therefore we need to pass to `hyper`'s
/// `Server` an executor and a way to perform the spawning.
///
/// We use `spawn_local` since we want each worker thread to be able to spawn !Send futures.
#[derive(Clone, Copy, Debug)]
struct LocalExec;

impl<F> hyper::rt::Executor<F> for LocalExec
where
    F: std::future::Future + 'static, // no `Send`
{
    fn execute(&self, fut: F) {
        // This will spawn into the currently running `LocalSet`.
        tokio::task::spawn_local(fut);
    }
}
