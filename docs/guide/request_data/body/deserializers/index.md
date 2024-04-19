# Overview

Deserializers transform the raw body into a Rust type.  
They take care of parsing, basic validation and security safeguards.
They're the family of extractors you'll use most often in your Pavex application.

Out of the box, Pavex supports the following encoding formats:

* [JSON](json.md)
* [URL encoded](url_encoded.md)

## Tower of abstractions

All deserializers are built on top of Pavex's [low-level primitives](../byte_wrappers.md).  
As such, they all share the same security safeguards, such as body size limits.  
Check out [the relevant guide](../byte_wrappers.md) for details on how to configure them.