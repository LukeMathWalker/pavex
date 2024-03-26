
# Overview

The [`ConnectionInfo`][ConnectionInfo] is a collection of data relating to the underlying HTTP connection 
of a request.

It can be used to perform business logic based on information such as [`ConnectionInfo::peer_addr`] 
which returns the "peer address". Be warned, the peer address should not be treated as the clients IP 
address - see <https://adam-p.ca/blog/2022/03/x-forwarded-for/> for more details.

## Injection

Inject [`ConnectionInfo`][ConnectionInfo] to access the peer address via its [`peer_addr`][RequestHead::peer_addr] 
method:

--8<-- "doc_examples/guide/request_data/connection/project-connection_peer.snap"

[ConnectionInfo]: ../../../api_reference/pavex/connection/struct.ConnectionInfo.html
[ConnectionInfo::peer_addr]: ../../../api_reference/pavex/connection/struct.ConnectionInfo.html#method.peer_addr