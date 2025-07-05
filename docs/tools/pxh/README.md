# Embedded Snippets

Documentation must be **trustworthy**. One outdated code example is enough to undermine the user's confidence in the whole library.

`phx` was built to ensure the code snippets on [pavex.dev](https://pavex.dev) are always accurate and up-to-date.

## How does it work?

All folders with an `example.yml` file are treated as an **example project** and scanned for embedded code snippets.\
Snippets are extracted from the source and stored in a .snap file named after them, which is then imported by our documentation
using the [`pymdown.snippets` plugin](https://facelessuser.github.io/pymdown-extensions/extensions/snippets/).

The .snap files are stored next to the `example.yml` file.

## Defining a snippet

There are two ways to declare an embedded snippet: file-wide and region-based.\
Prefer the file-wide format if _most_ of the file is meant to be part of the snippet.

### File-wide Snippets

The snippet is defined using a top-level comment, `//! px:<snippet_name>`.

```rust
//! px:example_snippet

fn main() {
    println!("Hello, world!");
}
```

The extracted snippet is:

````text
```rust
fn main() {
    println!("Hello, world!");
}
```
````

By default, the snippet includes all lines from the file.

### Region-based Snippets

The snippet is obtained by concatenating all lines between a `// px:<snippet_name>:start` and `// px:<snippet_name>:end` comment pair.\

```rust
fn main() {
    // px:example_snippet:start
    println!("Hello, world!");
    // px:example_snippet:end
}
```

The extracted snippet is:

````text
```rust
// [...]
    println!("Hello, world!");
// [...]
```
````

You can use multiple pairs of `// px:<snippet_name>:start` and `// px:<snippet_name>:end` to include multiple regions from
the same file in the snippet.

```rust
use std::io;

// px:example_snippet:start
fn greet() {
    println!("Hello!");
}
// px:example_snippet:end

// px:example_snippet:start
fn farewell() {
    println!("Goodbye!");
}
// px:example_snippet:end
```

The extracted snippet is:

````text
```rust
// [...]
fn greet() {
    println!("Hello!");
}

fn farewell() {
    println!("Goodbye!");
}
```
````

### Overlapping Snippets

If the regions for different snippets overlap, the snippet-related comments are automatically stripped from the extracted snippet.\
For example:

```rust
// px:full:start
// px:signature:start
fn greet() {
    println!("Hello!"); // px:signature:skip
}
// px:signature:end
// px:full:end
```

Extracts these two snippets:

- Signature:
  ````text
  ```rust
  fn greet() {
    // [...]
  }
  ```
  ````
- Full: 
  ````text
  ```rust
  fn greet() {
      println!("Hello!");
  }
  ```
  ````

### Skipping Lines

If you want to exclude some lines, you can either use:

- `// px:<snippet_name>:skip` trailing comments.\
  For example:
  ```rust
  //! px:example_snippet
  
  fn main() {
      println!("Hello, world!");
      println!("Skipped line!"); // px:example_snippet:skip
  }
  ```
  extracts:
  ````text
  ```rust
  fn main() {
      println!("Hello, world!");
      // [...]
  }
  ```
  ````

- `// px:<snippet_name>:skip:start` and `// px:<snippet_name>:skip:end` comment pairs.\
  For example:
  ```rust
  // px:example_snippet:start
  fn main() {
      // px:example_snippet:skip:start
      println!("Hello, world!");
      println!("Skipped line!"); 
      // px:example_snippet:skip:end
  }
  // px:example_snippet:end
  ```
  extracts:
  ````text
  ```rust
  fn main() {
      // [...]
  }
  ```
  ````

If no other snippets are defined in the same file, you can omit the snippet name in skip comments—i.e. `// px::skip`, 
`// px::skip:start` and `// px::skip:end` will be accepted.  
You can also omit the snippet name if you want the marked lines to be skipped by all snippets that would include them.

### Highlighting lines

You can highlight lines by adding `// px:<snippet_name>:hl` trailing comments.\
For example:

```rust
//! px:example_snippet

fn main() {
    println!("Hello, world!");
    println!("Highlighted line!"); // px:example_snippet:hl
}
```
extracts:
````text
```rust hl_lines="3"
fn main() {
    println!("Hello, world!");
    println!("Highlighted line!");
}
```
````

You can use `//px::hl` to highlight the corresponding line in all snippets that include it.

### Annotations

You can include an anchor in your snippet using a `/* px:<snippet_name>::ann::<number> /*` comment in the location where you want the anchor to be placed.

For example:

```rust
//! px:example_snippet

fn greet(/* px::ann:1 */ name: String) {
    println!("Hello, {name}!");
}
```
extracts:
````text
```rust
fn greet(/* (1)! */ name: String) {
    println!("Hello, world!");
}
```
````

The extracted syntax is the one expected for [annotations by Material for MkDocs](https://squidfunk.github.io/mkdocs-material/reference/annotations/).

You can use `// px::ann::<number>` if the anchor should be included in all snippets that cover that comment.

### Constraints

Snippet names can only contain underscores and alphanumeric ASCII characters. No spaces, no punctuation, no special characters.
