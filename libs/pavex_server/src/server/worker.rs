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
pub(super) struct Worker {
    connection_inbox: tokio::sync::mpsc::Receiver<TcpStream>,
    id: usize,
}

impl Worker {
    /// Configure a new worker without spawning it.
    ///
    /// `max_queue_length` is the maximum number of connections that can be queued up for this
    /// worker.
    pub(super) fn new(id: usize, max_queue_length: usize) -> (Self, WorkerHandle) {
        let (connection_outbox, connection_inbox) = tokio::sync::mpsc::channel(max_queue_length);
        let self_ = Self {
            connection_inbox,
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
        while let Some(_) = self.connection_inbox.recv().await {
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
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to build single-threaded Tokio runtime for worker thread")
                    .block_on(self.run());
            })
            .context("Failed to spawn worker thread")
    }
}
