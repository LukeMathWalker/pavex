#[test]
fn macro_ui_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/*/success/*.rs");
    t.compile_fail("tests/*/fail/*.rs");
}
