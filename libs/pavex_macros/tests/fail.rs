// `trybuild` tests are incredibly slow to compile on Windows, so we skip them
// to avoid slowing down the CI pipeline. They still run on Linux and macOS,
// and the outcome is not really platform-dependent.
#[cfg(not(target_os = "windows"))]
#[test]
fn fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fail/*.rs");
}
