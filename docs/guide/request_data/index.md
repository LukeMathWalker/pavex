# Overview

Pavex provides a comprehensive toolkit for extracting structured data out of the incoming request.  
We refer to these types as **extractors**.

**Extractors take away the burden of writing boilerplate code** for common tasks such as 
parsing query parameters, parsing path parameters, enforcing body size limits, etc.  
You'll feel **more productive** working on a Pavex application
if you know what first-party extractors are available and when they should be used.

## Guide structure

The guide is organised by **data source**: path, query, headers, body.  
Each section contains a list of extractors for that data source, with a brief description of their purpose and usage.

You can either read the guide from start to finish,
or jump to the section you're interested in on a need-to-know basis.  

## There is no magic

There is nothing special about Pavex's first-party extractors.
You could write your own versions of them, if you wanted to,
building on top of Pavex's [framework primitives](../dependency_injection/core_concepts/framework_primitives.md).  
Check out the ["Dependency injection"](../dependency_injection/index.md) guide for more information.