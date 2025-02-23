# Request data

The representation of an incoming request on the wire is often different from
the representation that your domain logic expects as input.
The raw request data has to go through **parsing** and **validation** before it's ready
to be processed.

Pavex can help.\
By the end of this guide, you'll have a solid understanding of the toolkit that Pavex provides
to extract structured data out of the incoming request.\
We'll start by looking at the types for the [raw incoming request](wire_data.md).
We'll then cover **extractors**, the mechanism used by Pavex to **take away the burden of writing
boilerplate code** for common tasks such as parsing query parameters, parsing path parameters,
enforcing body size limits, etc.
