use camino::Utf8Path;
use insta::assert_snapshot;
use pxh::snippets::*;

fn extract_rs_snippets(content: &str) -> anyhow::Result<Vec<Snippet>> {
    extract_embedded_snippets_from_content(content, FileKind::Rust)
}

fn extract_toml_snippets(content: &str) -> anyhow::Result<Vec<Snippet>> {
    extract_embedded_snippets_from_content(content, FileKind::Toml)
}

#[test]
fn test_file_wide_snippet() {
    let content = r#"//! px:example_snippet

fn main() {
    println!("Hello, world!");
}"#;

    let snippets = extract_rs_snippets(content).unwrap();
    assert_eq!(snippets.len(), 1);
    assert_eq!(snippets[0].name, "example_snippet");

    let rendered = snippets[0].render(Utf8Path::new("test.rs"));
    assert_snapshot!(rendered, @r###"
    ```rust
    fn main() {
        println!("Hello, world!");
    }
    ```
    "###);
}

#[test]
fn test_region_based_snippet() {
    let content = r#"fn main() {
    // px:example_snippet:start
    println!("Hello, world!");
    // px:example_snippet:end
}"#;

    let snippets = extract_rs_snippets(content).unwrap();
    assert_eq!(snippets.len(), 1);
    assert_eq!(snippets[0].name, "example_snippet");

    let rendered = snippets[0].render(Utf8Path::new("test.rs"));
    assert_snapshot!(rendered, @r###"
    ```rust
    // [...]
    println!("Hello, world!");
    ```
    "###);
}

#[test]
fn test_multiple_regions() {
    let content = r#"use std::io;

// px:example_snippet:start
fn greet() {
    println!("Hello!");
}
// px:example_snippet:end

fn other() {
    println!("Hello!");
}

// px:example_snippet:start
fn farewell() {
    println!("Goodbye!");
}
// px:example_snippet:end"#;

    let snippets = extract_rs_snippets(content).unwrap();
    assert_eq!(snippets.len(), 1);

    let rendered = snippets[0].render(Utf8Path::new("test.rs"));
    assert_snapshot!(rendered, @r###"
    ```rust
    // [...]
    fn greet() {
        println!("Hello!");
    }
    // [...]
    fn farewell() {
        println!("Goodbye!");
    }
    ```
    "###);
}

#[test]
fn test_skip_single_line() {
    let content = r#"//! px:example_snippet

fn main() {
    println!("Hello, world!");
    println!("Skip this line!"); // px:example_snippet:skip
}"#;

    let snippets = extract_rs_snippets(content).unwrap();
    assert_eq!(snippets.len(), 1);

    let rendered = snippets[0].render(Utf8Path::new("test.rs"));
    assert_snapshot!(rendered, @r###"
    ```rust
    fn main() {
        println!("Hello, world!");
        // [...]
    }
    ```
    "###);
}

#[test]
fn test_skip_region() {
    let content = r#"// px:example_snippet:start
fn main() {
    // px:example_snippet:skip:start
    println!("Skip this");
    println!("And this");
    // px:example_snippet:skip:end
    println!("Keep this");
}
// px:example_snippet:end"#;

    let snippets = extract_rs_snippets(content).unwrap();
    assert_eq!(snippets.len(), 1);

    let rendered = snippets[0].render(Utf8Path::new("test.rs"));
    assert_snapshot!(rendered, @r###"
    ```rust
    fn main() {
        // [...]
        println!("Keep this");
    }
    ```
    "###);
}

#[test]
fn test_highlight_lines() {
    let content = r#"//! px:example_snippet

fn main() {
    println!("Normal line");
    println!("Highlighted line"); // px:example_snippet:hl
}"#;

    let snippets = extract_rs_snippets(content).unwrap();
    let rendered = snippets[0].render(Utf8Path::new("test.rs"));
    assert_snapshot!(rendered, @r###"
    ```rust hl_lines="3"
    fn main() {
        println!("Normal line");
        println!("Highlighted line");
    }
    ```
    "###);
}

#[test]
fn test_annotations() {
    let content = r#"//! px:example_snippet

fn greet(/* px::ann:1 */ name: String) {
    println!("Hello, {name}!");
}"#;

    let snippets = extract_rs_snippets(content).unwrap();
    assert_eq!(snippets.len(), 1);

    let rendered = snippets[0].render(Utf8Path::new("test.rs"));
    assert_snapshot!(rendered, @r###"
    ```rust hl_lines="1"
    fn greet(/* (1)! */ name: String) {
        println!("Hello, {name}!");
    }
    ```
    "###);
}

#[test]
fn test_overlapping_snippets() {
    let content = r#"// px:full:start
// px:signature:start
fn greet() {
    println!("Hello!"); // px:signature:skip
}
// px:signature:end
// px:full:end"#;

    let snippets = extract_rs_snippets(content).unwrap();
    assert_eq!(snippets.len(), 2);

    // Find each snippet by name
    let full_snippet = snippets.iter().find(|s| s.name == "full").unwrap();
    let sig_snippet = snippets.iter().find(|s| s.name == "signature").unwrap();

    let full_rendered = full_snippet.render(Utf8Path::new("test.rs"));
    let sig_rendered = sig_snippet.render(Utf8Path::new("test.rs"));

    assert_snapshot!(full_rendered, @r###"
    ```rust
    fn greet() {
        println!("Hello!");
    }
    ```
    "###);
    assert_snapshot!(sig_rendered, @r###"
    ```rust
    fn greet() {
        // [...]
    }
    ```
    "###);
}

#[test]
#[should_panic]
fn test_invalid_snippet_names() {
    let content = r#"//! px:invalid-name
//! px:valid_name

fn main() {}"#;

    extract_rs_snippets(content).unwrap();
}

#[test]
fn test_generic_skip_directive() {
    let content = r#"//! px:snippet1
//! px:snippet2

fn main() {
    println!("Both snippets see this");
    println!("Neither snippet sees this"); // px::skip
}"#;

    let snippets = extract_rs_snippets(content).unwrap();
    assert_eq!(snippets.len(), 2);

    let snippet1 = snippets.iter().find(|s| s.name == "snippet1").unwrap();
    let snippet2 = snippets.iter().find(|s| s.name == "snippet2").unwrap();

    assert_snapshot!(snippet1.render(Utf8Path::new("test.rs")), @r###"
    ```rust
    fn main() {
        println!("Both snippets see this");
        // [...]
    }
    ```
    "###);
    assert_snapshot!(snippet2.render(Utf8Path::new("test.rs")), @r###"
    ```rust
    fn main() {
        println!("Both snippets see this");
        // [...]
    }
    ```
    "###);
}

#[test]
fn test_ellipsis_at_beginning() {
    let content = r#"fn setup() {
// Some setup code
}

// px:example:start
fn main() {
    println!("Hello");
}
// px:example:end"#;

    let snippets = extract_rs_snippets(content).unwrap();
    assert_eq!(snippets.len(), 1);

    let rendered = snippets[0].render(Utf8Path::new("test.rs"));
    assert_snapshot!(rendered, @r###"
    ```rust
    // [...]
    fn main() {
        println!("Hello");
    }
    ```
    "###);
}

#[test]
fn test_toml_file_snippet() {
    let content = r#"[package]
name = "example"

# px:deps:start
[dependencies]
serde = "1.0"
# px:deps:end"#;

    let snippets = extract_toml_snippets(content).unwrap();
    assert_eq!(snippets.len(), 1);

    let rendered = snippets[0].render(Utf8Path::new("Cargo.toml"));
    assert_snapshot!(rendered, @r###"
    ```toml
    # [...]
    [dependencies]
    serde = "1.0"
    ```
    "###);
}
