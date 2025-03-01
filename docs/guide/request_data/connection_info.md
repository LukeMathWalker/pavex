# Connection info

[`ConnectionInfo`][ConnectionInfo] groups together information about the HTTP connection
used to transmit the request you're currently processing.

[`ConnectionInfo`][ConnectionInfo] gives you access to the peer address of the client that sent the request, via
the [`ConnectionInfo::peer_addr`][ConnectionInfo::peer_addr] method.\
Many applications include the peer address in their request logs, for example.

!!! warning "Security implications"

    The peer address should **not** be treated as the clients IP 
    address. Check out ["The perils of the 'real' client IP"](https://adam-p.ca/blog/2022/03/x-forwarded-for/) 
    by Adam Pritchard to learn more about the issues you might run into.

## Injection

Inject [`ConnectionInfo`][ConnectionInfo] to access the peer address via its [`peer_addr`][ConnectionInfo::peer_addr]
method:

--8<-- "doc_examples/guide/request_data/connection/project-connection_peer.snap"

[ConnectionInfo]: /api_reference/pavex/connection/struct.ConnectionInfo.html
[ConnectionInfo::peer_addr]: /api_reference/pavex/connection/struct.ConnectionInfo.html#method.peer_addr
