# Overview

The **body** of an HTTP request is the primary mechanism to transmit a payload over the wire.

For the HTTP protocol, the body is just a stream of bytes.
For an application, the body is a structured payload; it must be parsed and validated before it can be used.

Pavex provides tools at **different levels of abstraction** for working with the body of an HTTP request.

## Deserializers

Deserializers transform the body into a Rust type, taking care of parsing, basic validation and security safeguards.
They're the family of extractors you'll use most often in your Pavex application.

Out of the box, Pavex provides [JSON](json.md) and [URL encoded](url_encoded.md).

## Byte wrappers

[Byte wrappers](byte_wrappers.md) provide a safe(r) interface over the underlying stream of bytes.
They provide safeguards and conveniences, but they don't do any parsing.

## Raw access

[The raw stream of bytes](../wire_data.md#rawincomingbody), straight from the network to your code.
No parsing, no safeguards, no conveniences.
