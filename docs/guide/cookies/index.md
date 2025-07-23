# Cookies

[Cookies](https://developer.mozilla.org/en-US/docs/Web/HTTP/Cookies) are a mechanism
to attach state to an otherwise stateless protocol, HTTP.
They are often used by web applications to manage authenticated sessions, shopping cart contents,
and other kinds of ephemeral user-specific data.

## What is a cookie?

A cookie is a key-value pair.\
E.g. `session_id=1234567890abcdef` is a cookie,
where `session_id` is the **cookie name** and `1234567890abcdef` is the **cookie value**.

## Where are cookies stored?

Cookies are stored on the client side, usually by a browser.\
Cookies are passed back and forth between the client and the server
using the [Cookie](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Cookie)
and [Set-Cookie](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie) headers.

The [Cookie header](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Cookie) is used by clients
to send relevant cookies to the server when they issue requests.\
The [Set-Cookie header](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie), instead, is used by the
server to alter the state on the client-side, either by creating new cookies,
removing existing ones, or updating their attributes.

## Cookie attributes

On top of the name and value, cookies can have a number of **attributes** that control their behavior:
`Path`, `Domain`, `Expires`, `Max-Age`, `Secure`, `HttpOnly`, `SameSite`, etc.\
Those attributes are used:

- by the client, to determine if a cookie should be sent back to the server or not (e.g. `Path`, `Domain`, `SameSite`, `Secure`)
- by the server, to determine how long the cookie should be stored on the client-side (e.g. `Expires`, `Max-Age`)
  and what restrictions should be applied to it (e.g. `Secure`, `HttpOnly`).

Refer to the [MDN documentation](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie#attributes)
for more details on each attribute.

## Cookies in Pavex

Pavex provides first-party support for manipulating cookies.\
Check out the ["Installation"](installation.md) section
to learn how to set up the machinery required to work with cookies.\
Once everything is in place, you can start using cookies in your application:

- Check out ["Request cookies"](request_cookies.md) to learn how to access cookies sent by the client.
- Check out ["Response cookies"](response_cookies.md) to learn how to attach cookies to the response,
  to either set new cookies, update existing ones, or delete them.

[Blueprint]: /api_reference/pavex/struct.Blueprint.html
[CookieKit]: /api_reference/pavex/cookie/struct.CookieKit.html
[ProcessorConfig]: /api_reference/pavex/cookie/struct.ProcessorConfig.html
[ProcessorConfig::default]: /api_reference/pavex/cookie/struct.ProcessorConfig.html#method.default
[default settings]: /api_reference/pavex/cookie/struct.ProcessorConfig.html#fields
[RequestCookies]: /api_reference/pavex/cookie/struct.RequestCookies.html
[RequestCookies::get]: /api_reference/pavex/cookie/struct.RequestCookies.html#method.get
[RequestCookies::get_all]: /api_reference/pavex/cookie/struct.RequestCookies.html#method.get_all
[ResponseCookie::new]: /api_reference/pavex/cookie/struct.ResponseCookie.html#method.new
[ResponseCookies]: /api_reference/pavex/cookie/struct.ResponseCookies.html
[ResponseCookies::insert]: /api_reference/pavex/cookie/struct.ResponseCookies.html#method.insert
[RemovalCookie]: /api_reference/pavex/cookie/struct.RemovalCookie.html
[response_cookie_injector]: /api_reference/pavex/cookie/struct.CookieKit.html#method.response_cookie_injector
