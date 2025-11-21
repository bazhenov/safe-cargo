use std::process::Command;

#[test]
fn cli() {
    // We need to clean untracked files in test cargo projects so that tests will be reproducible
    //
    // One example where it is important is Cargo.lock file which should be absent from cargo projects.
    // Cargo will try to write it on first build. If it is already present it might mask possible errors
    // if `cargo safe` deny such writes.
    Command::new("git")
        .arg("clean")
        .arg("-fdx")
        .arg("--")
        .arg("tests")
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    trycmd::TestCases::new()
        .case("tests/positive/*.toml")
        .case("tests/negative/*.toml")
        .default_bin_name("cargo-safe");
}
