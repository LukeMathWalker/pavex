# Sessions

Sessions allow web applications to store user information across multiple requests.\
They're used to manage authentication, shopping cart contents, and other kinds of
ephemeral user-specific data.

## Anatomy of a session

Each session has two components: a **client-side cookie** and a **server-side state**.
They are linked together by a unique identifier, the **session ID**.

The client-side cookie allows the server to identify the session across multiple requests. It
can also be used to store small amounts of data, the **client-side session state**.

The server-side state is stored in a **session storage backend**. It contains the bulk of the
session data, and it is identified by the session ID.
The storage backend can be a traditional SQL database (e.g. PostgreSQL), an in-memory database
(e.g. Redis), or any other storage system. Pavex ships with built-in support for the most
popular databases, but you can easily add support for your favorite one if it's not already
included.

## References

Further reading on sessions:

- [RFC 6265: HTTP State Management Mechanism](https://datatracker.ietf.org/doc/html/rfc6265)
- [OWASP's session management cheat-sheet](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html)
