# Request cookies

Inject [`&RequestCookies`][RequestCookies] into your components to access the cookies sent by the client
alongside the incoming request.
You can then retrieve a cookie by name using the [`get`][RequestCookies::get] method:

--8<-- "doc_examples/guide/cookies/request_cookies/project-inject.snap"

## Multiple cookies with the same name

In [some scenarios](https://stackoverflow.com/questions/4056306/how-to-handle-multiple-cookies-with-the-same-name/24214538#24214538), 
the client might send multiple cookies with the same name.  
[`get`][RequestCookies::get] will return the first one it finds.
If that's not what you want, you can use [`get_all`][RequestCookies::get_all] to retrieve all of them:

--8<-- "doc_examples/guide/cookies/request_cookies/project-multiple.snap"

[RequestCookies]: ../../api_reference/pavex/cookie/struct.RequestCookies.html
[RequestCookies::get]: ../../api_reference/pavex/cookie/struct.RequestCookies.html#method.get
[RequestCookies::get_all]: ../../api_reference/pavex/cookie/struct.RequestCookies.html#method.get_all