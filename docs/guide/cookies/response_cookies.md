# Response cookies

To attach cookies to the response, you need to insert them into [`ResponseCookies`][ResponseCookies].  
The [`response_cookie_injector`][response_cookie_injector] middleware in [`CookieKit`][CookieKit] will take care of adding a `Set-Cookie`
header to the response for each cookie inside [`ResponseCookies`][ResponseCookies].

## Add a response cookie

Inject `&mut ResponseCookies` into the component that needs to set a cookie:

--8<-- "doc_examples/guide/cookies/response_cookies/project-insert.snap"

You can use [`ResponseCookie::new`][ResponseCookie::new] to start building a new cookie.
It exposes multiple `set_*` methods to configure the cookie's properties: `Path`, `Domain`, `Secure`, `HttpOnly`, etc.

!!! note

    You can only inject mutable references into [request handlers](../routing/request_handlers.md),
    [pre-processing middlewares](../middleware/pre_processing.md), and [post-processing middlewares](../middleware/post_processing.md). 
    As a result, you can only set cookies in those components.
    Check out ["No mutations"](../dependency_injection/constructors.md#no-mutations) for more information
    on the rationale.

## Remove a client-side cookie

If you want to tell the client to delete a cookie, you need to insert a [`RemovalCookie`][RemovalCookie]
into [`ResponseCookies`][ResponseCookies]:

--8<-- "doc_examples/guide/cookies/response_cookies/project-delete.snap"

The client will receive a `Set-Cookie` header with the cookie name and an empty value,
along with an expiration date in the past.  
You need to make sure that the `Path` and `Domain` properties on the [`RemovalCookie`][RemovalCookie] match the ones
set on the client-side cookie you want to delete.

[CookieKit]: ../../api_reference/pavex/cookie/struct.CookieKit.html
[ResponseCookie::new]: ../../api_reference/pavex/cookie/struct.ResponseCookie.html#method.new
[ResponseCookies]: ../../api_reference/pavex/cookie/struct.ResponseCookies.html
[ResponseCookies::insert]: ../../api_reference/pavex/cookie/struct.ResponseCookies.html#method.insert
[RemovalCookie]: ../../api_reference/pavex/cookie/struct.RemovalCookie.html
[response_cookie_injector]: ../../api_reference/pavex/cookie/struct.CookieKit.html#structfield.response_cookie_injector
