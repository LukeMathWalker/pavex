// `trybuild` tests are incredibly slow to compile on Windows, so we skip them
// to avoid slowing down the CI pipeline. They still run on Linux and macOS,
// and the outcome is not really platform-dependent.
#[cfg(not(target_os = "windows"))]
#[test]
fn fail() {
    let t = trybuild::TestCases::new();

    // Tests for `#[PathParams]`
    t.pass("tests/path_params/success/*.rs");
    t.compile_fail("tests/path_params/fail/*.rs");

    // Tests for `from!`
    t.pass("tests/from/success/*.rs");
    t.compile_fail("tests/from/fail/*.rs");

    // Tests for `constructor!`
    t.pass("tests/constructor/success/*.rs");
    t.compile_fail("tests/constructor/fail/*.rs");

    // Tests for `wrap!`
    t.pass("tests/wrap/success/*.rs");
    t.compile_fail("tests/wrap/fail/*.rs");
}
