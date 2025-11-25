#[test]
fn cli() {
    trycmd::TestCases::new()
        .case("tests/positive/*.toml")
        .case("tests/negative/*.toml")
        .default_bin_name("safe-cargo");
}
