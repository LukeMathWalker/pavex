# Routing

A **route** processes incoming requests for a given **method** and [**path pattern**](path_patterns.md).
Pavex takes care, at runtime, of routing incoming requests to the most appropriate route.

As your application grows, you may benefit from Pavex's more advanced routing features:

- [**Fallbacks**], to customize the response returned when no route matches.
- [**Path prefixes**](path_prefixes.md), to reduce repetition in your route definitions.
- [**Domain guards**](domain_guards.md), to serve different content based on the domain being requested.

## Defining a route

Pavex provides attributes for the most common HTTP methods: [`#[get]`][get_attr], [`#[post]`][post_attr], [`#[put]`][put_attr], [`#[patch]`][patch_attr], [`#[delete]`][delete_attr], [`#[head]`][head_attr], and [`#[options]`][options_attr].

--8<-- "docs/examples/routing/core_concepts/multiple_named_parameters.snap"

Use [`#[route]`][route_attr], instead, to define routes that match [multiple methods][multiple_methods], [non-standard methods][non_standard_methods] or [arbitrary methods][arbitrary_methods].

## Requirements

Routes must return, as output, a type that implements the [`IntoResponse`][IntoResponse] trait.\
Routes can fail, too. A fallible route will return `Result<T, E>`, where `T` implements [`IntoResponse`][IntoResponse] and `E` is an error type.

Other than that, you have a lot of freedom in how you define your routes:

- [They can be free functions or methods.](/guide/attributes/functions_and_methods.md)
- [They can be synchronous or asynchronous.](/guide/attributes/sync_or_async.md)
- [They can take additional input parameters, leaning on Pavex's dependency injection system.](/guide/dependency_injection/index.md)

## Registration

Use [`Blueprint::routes`][Blueprint::routes] to register in bulk all the routes defined in the current crate:

--8<-- "docs/examples/routing/core_concepts/registration.snap"

1. You can also import routes from [other crates][import_other_crates] or [specific modules][import_specific_modules].

Alternatively, register routes one by one using [`Blueprint::route`][Blueprint::route]:

--8<-- "docs/examples/routing/core_concepts/register_one.snap"

1. `FORMAL_GREET` is a strongly-typed constant generated by the [`#[get]`][get_attr] attribute on the `formal_greet` function.\
   Check out the documentation on [component ids](/guide/attributes/component_id.md) for more details.

### Position matters

Be careful when registering routes: their position relative to [middlewares](/guide/middleware/execution_order.md) and [error observers](/guide/errors/error_observers.md#position-matters) determines if they are affected by them, or not.

[Blueprint]: /api_reference/pavex/struct.Blueprint.html
[Blueprint::route]: /api_reference/pavex/struct.Blueprint.html#method.route
[Blueprint::routes]: /api_reference/pavex/struct.Blueprint.html#method.routes
[IntoResponse]: /api_reference/pavex/trait.IntoResponse.html
[**Fallbacks**]: /api_reference/pavex/struct.Blueprint.html#method.fallback
[get_attr]: /api_reference/pavex/attr.get.html
[post_attr]: /api_reference/pavex/attr.post.html
[put_attr]: /api_reference/pavex/attr.put.html
[patch_attr]: /api_reference/pavex/attr.patch.html
[delete_attr]: /api_reference/pavex/attr.delete.html
[head_attr]: /api_reference/pavex/attr.head.html
[options_attr]: /api_reference/pavex/attr.options.html
[route_attr]: /api_reference/pavex/attr.route.html
[import_other_crates]: /api_reference/pavex/struct.Blueprint.html#dependencies
[import_specific_modules]: /api_reference/pavex/struct.Blueprint.html#specific-modules
[multiple_methods]: /api_reference/pavex/attr.route.html#example-multiple-methods
[non_standard_methods]: /api_reference/pavex/attr.route.html#example-non-standard-method
[arbitrary_methods]: /api_reference/pavex/attr.route.html#example-arbitrary-methods
