#[test]
fn pass() {
    let t = trybuild::TestCases::new();
    t.pass("tests/success/*.rs");
}
