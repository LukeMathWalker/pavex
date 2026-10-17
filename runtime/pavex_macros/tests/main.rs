#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/*/success/*.rs");
    t.compile_fail("tests/*/fail/*.rs");
}
