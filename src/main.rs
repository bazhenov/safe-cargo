use safe_cargo::prepare_profile;
use std::{
    env, fs, io,
    path::PathBuf,
    process::{Command, ExitCode},
};

/// This crate is available only on macOS because it relies on `sandbox-exec` cli tool
#[cfg(target_os = "macos")]
fn main() -> Result<ExitCode, io::Error> {
    use clap::{Arg, ArgAction, command};

    const CARGO_TARGET_DIR: &str = "CARGO_TARGET_DIR";

    let cmd = command!().args(&[Arg::new("dump-profile")
        .short('d')
        .long("dump-profile")
        .help("Print a sandbox profile contents")
        .action(ArgAction::SetTrue)]);

    let args = env::args().collect::<Vec<_>>();
    let Args { cargo_args, args } = split_cargo_args(args);
    let options = cmd.get_matches_from(args);

    let Some(workspace_path) = env::current_dir().and_then(find_workspace_dir)? else {
        panic!("Unable to find cargo workspace directory");
    };

    let target_path = env::var(CARGO_TARGET_DIR)
        .map(PathBuf::from)
        .ok()
        .unwrap_or_else(|| workspace_path.join("target"));

    let sandbox_path = target_path.join("cargo-safe");
    if !fs::exists(&sandbox_path)? {
        fs::create_dir_all(&sandbox_path)?;
    }

    let sandbox_profile = prepare_profile(&workspace_path, &sandbox_path)?;

    if options.get_flag("dump-profile") {
        println!("{}", sandbox_profile);
        return Ok(ExitCode::SUCCESS);
    }

    let profile_path = sandbox_path.join("profile.sb");
    fs::write(&profile_path, sandbox_profile.to_string())?;

    let result = Command::new("sandbox-exec")
        .arg("-f")
        .arg(profile_path)
        .arg("cargo")
        .args(cargo_args)
        .env(CARGO_TARGET_DIR, sandbox_path.join("target"))
        .env("CARGO_HOME", sandbox_path.join("cargo"))
        .spawn()?
        .wait()?;

    let code = match result.code() {
        Some(0) => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    };
    Ok(code)
}

struct Args<T: AsRef<str>> {
    /// arguments of `safe-cargo`
    args: Vec<T>,
    /// arguments passed to a cargo command
    cargo_args: Vec<T>,
}

fn split_cargo_args<T: AsRef<str>>(mut args: Vec<T>) -> Args<T> {
    let double_dash_idx = args
        .iter()
        .enumerate()
        .find(|(_, i)| i.as_ref() == "--")
        .map(|(idx, _)| idx);
    if let Some(double_dash_idx) = double_dash_idx {
        let cargo_args = args.split_off(double_dash_idx + 1);
        // Removing double dash from the list of our arguments
        args.pop();
        Args { cargo_args, args }
    } else {
        Args {
            cargo_args: args,
            args: vec![],
        }
    }
}

fn find_workspace_dir(path: PathBuf) -> io::Result<Option<PathBuf>> {
    let mut path = fs::canonicalize(path)?;
    if path.join("Cargo.toml").exists() {
        return Ok(Some(path));
    }
    while path.pop() {
        if path.join("Cargo.toml").exists() {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_split_cargo_args() {
        macro_rules! assert_split {
            ([$($args:expr),*] => [$($expected_args:expr),*], [$($expected_cargo_args:expr),*]) => {
                let Args { args, cargo_args } = split_cargo_args::<&str>(vec![$($args),*]);
                assert_eq!(args, &[$($expected_args),*] as &[&str], "safe-cargo arguments are not valid");
                assert_eq!(cargo_args, &[$($expected_cargo_args),*] as &[&str], "cargo arguments are not valid");
            };
        }

        assert_split!([] => [], []);
        assert_split!(["build"] => [], ["build"]);
        assert_split!(["+nightly", "build"] => [], ["+nightly", "build"]);
        assert_split!(["--", "+nightly", "build"] => [], ["+nightly", "build"]);
        assert_split!(["--dump", "--", "+nightly", "build"] => ["--dump"], ["+nightly", "build"]);
        assert_split!(["--dump", "--"] => ["--dump"], []);
        assert_split!(["--dump", "--", "run", "--", "-a"] => ["--dump"], ["run", "--", "-a"]);
    }
}
