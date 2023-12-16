# Testing

All your testing, so far, has been manual: you've been launching the application and issuing requests to it with `curl`.
Let's move away from that: it's time to write some automated tests!

## Black-box testing

The preferred way to test a Pavex application is to treat it as a black box: you should only test the application
through its HTTP interface. This is the most realistic way to test your application: it's how your users will
interact with it, after all.

The template project includes a reference example for the `/api/ping` endpoint:

--8<-- "doc_examples/quickstart/09-ping_test.snap"

1. `TestApi` is a helper struct that provides a convenient interface to interact with the application.  
   It's defined in `demo_server/tests/helpers.rs`.
2. `TestApi::spawn` starts a new instance of the application in the background.
3. `TestApi::get_ping` issues an actual `GET /api/ping` request to the application.

## Add a new integration test

Let's write a new integration test to verify the behaviour on the happy path for `GET /api/greet/:name`:

--8<-- "doc_examples/quickstart/09-new_test_module.snap"

--8<-- "doc_examples/quickstart/09-greet_test.snap"

It follows the same pattern as the `ping` test: it spawns a new instance of the application, issues a request to it
and verifies that the response is correct.  
Let's complement it with a test for the unhappy path as well: requests with a malformed `User-Agent` header should be
rejected.

--8<-- "doc_examples/quickstart/10-greet_test.snap"

`cargo px test` should report three passing tests now. As a bonus exercise, try to add a test for the case where the
`User-Agent` header is missing.

