# Sync or async?

You can use Pavex's attributes on all functions and methods, no matter if they are synchronous or asynchronous.

Be careful, though!
Synchronous components will **block the current thread** until they execute to completion.

That's not a concern if you are performing an operation that's **guaranteed** to be fast
(e.g. converting an error into a response).
It becomes an issue if you're doing work that's **potentially** slow.
In the wild, there are two main categories of "potentially slow" operations:

- Input/output (I/O) operations (e.g. reading from a file, querying a database, etc.)
- CPU-intensive computations (e.g. hashing a password, parsing a large file, etc.)

As a rule of thumb:

| I/O | CPU-intensive | Function type | Notes                                                                                                                              |
| --- | ------------- | ------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| Yes | No            | Async         | Use async libraries for the I/O portion. If the I/O interface is synchronous, use [`tokio::task::spawn_blocking`][spawn_blocking]. |
| No  | Yes           | Async         | Use [`tokio::task::spawn_blocking`][spawn_blocking] for the CPU-intensive portion.                                                 |
| Yes | Yes           | Async         | See above.                                                                                                                         |
| No  | No            | Sync          | You can also make it asynchronous, it doesn't matter.                                                                              |

Check out [Alice Rhyl's excellent article](https://ryhl.io/blog/async-what-is-blocking/) to learn more about what "blocking" means in the world of asynchronous Rust.

[spawn_blocking]: https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html
