# Session data

The [`Session`][Session] type is the main interface to work with sessions in Pavex.\
You can think of the session state as a collection of key-value pairs. The keys are strings, the values are
arbitrary—e.g. strings, numbers, or even complex data structures[^internal-representation].

Let's go through a few examples to get familiar with the basic operations you can perform.

## Storing data

Use [`insert`][insert] to store an entry in the server-side state of your session:

--8<-- "docs/examples/sessions/postgres/server_insert.snap"

1. Pavex knows how to inject a `&mut Session` or a `&Session` as an input parameter
   thanks to [your `pavex_session` import](installation.md#blueprint).

In the example above, [`insert`][insert] will create a new `user.id` entry in the session state.
If there is already a `user.id` entry, it'll overwrite it.

### Complex objects

You're not limited to storing simple values. You can store complex types like structs
or enums as well, if they are serializable:

--8<-- "docs/examples/sessions/postgres/server_insert_struct.snap"

1. We derive `serde`'s `Serialize` and `Deserialize` traits to ensure the type
   can be serialized and deserialized.
2. [`insert`][insert] will return an error if the serialization step fails.

## Retrieving data

Use [`get`][get] to retrieve an entry from the server-side state of your session:

--8<-- "docs/examples/sessions/postgres/server_get.snap"

1. [`get`][get] doesn't modify the session state, it only reads from it.
   It is therefore enough to ask for a shared reference to the session,
   rather than a mutable one.
   If you need to both read and write to the session state, ask for a mutable reference.
2. [`get`][get] returns an `Option` because the key might not exist in the session state.
   We specify the type of the value we expect to get back so that [`get`]
   knows what to deserialize the value as, if it exists.
3. [`get`][get] may fail if the value is not of the expected type, thus failing
   the deserialization step.

### Complex objects

The process is exactly the same for more complex types. You just need to specify the type
you expect to get back:

--8<-- "docs/examples/sessions/postgres/server_get_struct.snap"

1. The extracted type must be deserializeable. That's why we derive `serde`'s `Deserialize` trait.
2. The type annotation tells [`get`][get] to deserialize the value as an `AuthInfo` instance,
   rather than a string, as in the previous example.

## Removing data

Use [`remove`][remove] to delete an entry from the server-side state of your session:

--8<-- "docs/examples/sessions/postgres/server_remove.snap"

1. You must add a type annotation to tell [`remove`][remove] what type of value you expect to get back.

Remove returns the entry that was removed, if it existed, or `None` otherwise.\
If you don't plan to use the removed value, you can invoke [`remove_raw`][remove_raw] instead:

--8<-- "docs/examples/sessions/postgres/server_remove_raw.snap"

It returns the raw entry, without trying to deserialize it. It spares you from having to specify the type.

## Regenerating the session ID

Your application may be required to regenerate the session ID
to prevent [session fixation attacks](https://owasp.org/www-community/attacks/Session_fixation).\
You can do this by calling [`cycle_id`][cycle_id]:

--8<-- "docs/examples/sessions/postgres/cycle_id.snap"

[`cycle_id`][cycle_id] doesn't change the session state in any way.

## Session invalidation

If you want to destroy the current session, call [`invalidate`][invalidate]:

--8<-- "docs/examples/sessions/postgres/invalidate.snap"

[`invalidate`][invalidate] will:

- remove the server-side state from the storage backend
- delete the session cookie on the client-side

It effectively ends the current session.\
All operations on the current session after invoking [`invalidate`][invalidate] will be ignored.

### Deletion without invalidation

There may be situations where you want to keep the session alive, but remove all data
from the server-side state. Use [`clear`][clear]:

--8<-- "docs/examples/sessions/postgres/server_clear.snap"

[`clear`][clear] removes all entries from the server-side state, leaving an empty record
in the storage backend.\
If you want to delete the server-side state entry completely, use [`delete`][delete]:

--8<-- "docs/examples/sessions/postgres/server_delete.snap"

[`delete`][delete] will remove the server-side state entry from the storage backend, but it won't
delete the session cookie on the client-side.

## Client-side state

As we discussed in the [introduction](index.md#anatomy-of-a-session), there are two types of session data:
the client-side state and the server-side state.\
All the examples above manipulate the server-side state, but there may be cases where you want to
store data in the client-side state to minimize the number of round-trips to the session storage
backend.

You can use [`client`][client] and [`client_mut`][client_mut] to perform the same operations on the client-side state:

--8<-- "docs/examples/sessions/postgres/client_ops.snap"

Keep in mind that the client-side state is stored inside the session cookie.\
It's not suitable for storing large amounts of data and it is inherently more exposed than its
server-side counterpart. Use it only for small, non-sensitive data.

[^internal-representation]: Internally, each value is stored as a JSON object. This means that
    you can store any type that can be serialized to (and deserialized from) JSON. In Rust terms,
    you can reason about the session state as if it were a `HashMap<String, serde_json::Value>`.

[Session]: /api_reference/pavex_session/struct.Session.html
[delete]: /api_reference/pavex_session/struct.Session.html#method.delete
[cycle_id]: /api_reference/pavex_session/struct.Session.html#method.cycle_id
[invalidate]: /api_reference/pavex_session/struct.Session.html#method.invalidate
[client]: /api_reference/pavex_session/struct.Session.html#method.client
[client_mut]: /api_reference/pavex_session/struct.Session.html#method.client_mut
[clear]: /api_reference/pavex_session/struct.Session.html#method.clear
[insert]: /api_reference/pavex_session/struct.Session.html#method.insert
[remove]: /api_reference/pavex_session/struct.Session.html#method.remove
[remove_raw]: /api_reference/pavex_session/struct.Session.html#method.remove_raw
[get]: /api_reference/pavex_session/struct.Session.html#method.get
