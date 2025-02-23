# Overview

The **body** of an HTTP request is the primary mechanism to transmit a payload over the wire.

For the HTTP protocol, the body is just a stream of bytes.\
For an application, the body is a structured payload.
It must be parsed and validated before it can be used.

Pavex provides tools at **different levels of abstraction** for working with the body of an HTTP request.
Going from the highest to the lowest level of abstraction, we have:

- [Deserializers](deserializers/index.md).
  They transform the body into a Rust type, taking care of parsing, basic validation and security safeguards.
- [Byte wrappers](byte_wrappers.md).
  A safe interface over the underlying stream of bytes.
  They provide safeguards and conveniences, but they don't do any parsing.
- [Raw access](../wire_data.md#rawincomingbody).
  The raw stream of bytes, straight from the network to your code.
  No parsing, no safeguards, no conveniences.
