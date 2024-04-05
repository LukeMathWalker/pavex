# Request cookies

Inject [`&RequestCookies`][RequestCookies] into your components to access the cookies sent by the client
alongside the incoming request.

// Code example

You can retrieve a cookie by name using the [`get`][RequestCookies::get] method.  
In some scenarios, the client might send multiple cookies with the same name.
You can use [`get_all`][RequestCookies::get_all] to retrieve all of them, if needed.

[RequestCookies]: ../../api_reference/pavex/cookie/struct.RequestCookies.html
[RequestCookies::get]: ../../api_reference/pavex/cookie/struct.RequestCookies.html#method.get
[RequestCookies::get_all]: ../../api_reference/pavex/cookie/struct.RequestCookies.html#method.get_all