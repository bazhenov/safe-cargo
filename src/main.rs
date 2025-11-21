use crate::seatbelt::{Filter, Operation, Profile, allow, deny};
use std::{collections::VecDeque, env, fs, io, path::Path, process::Command};

/// This crate is available only on macOS becuase it relies on `sandbox-exec` cli tool
#[cfg(target_os = "macos")]
fn main() -> Result<(), io::Error> {
    let mut args = env::args().collect::<VecDeque<_>>();

    // skipping program name
    let _program_name = args.pop_front();
    // skipping "safe" if command is called as "cargo safe"
    if let Some("safe") = args.front().map(String::as_str) {
        args.pop_front();
    }

    let Ok(workspace_path) = env::current_dir() else {
        panic!("Error reading current directory");
    };

    let sandbox_path = workspace_path.join(".sandbox");
    if !fs::exists(&sandbox_path)? {
        fs::create_dir_all(&sandbox_path)?;
    }

    let sandbox_profile = prepare_profile(&workspace_path, &sandbox_path)?;
    if args.iter().any(|o| *o == "--dump-profile") {
        println!("{}", sandbox_profile);
        return Ok(());
    }
    let profile_path = sandbox_path.join("profile.sb");
    fs::write(&profile_path, sandbox_profile.to_string())?;

    Command::new("sandbox-exec")
        .arg("-f")
        .arg(profile_path)
        .arg("cargo")
        .args(args)
        .env("CARGO_TARGET_DIR", sandbox_path.join("target"))
        .env("CARGO_HOME", sandbox_path.join("cargo"))
        .spawn()?
        .wait()?;
    Ok(())
}

fn prepare_profile(workspace: &Path, sandbox: &Path) -> Result<Profile, io::Error> {
    use Filter::*;
    use Operation::*;

    let path_items = if let Ok(path) = env::var("PATH") {
        path.split(":")
            .map(|p| Prefix(p.to_owned()))
            .collect::<Vec<_>>()
    } else {
        vec![]
    };

    let mut rules = vec![
        deny(Default, None),
        allow(ProcessAll, None),
        allow(SysctlRead, None),
        allow(MachLookup, None),
        allow(IpcPosixShmReadData, None),
        allow(UserPreferenceRead, None),
        allow(FileReadMetadata, vec![Prefix("/".into())]),
        allow(FileIoctl, vec![Literal("/dev/dtracehelper".into())]),
        allow(
            FileWriteAll,
            vec![
                Literal("/dev/dtracehelper".into()),
                Literal("/dev/null".into()),
                Literal("/dev/tty".into()),
            ],
        ),
        // allowing to read binaries from PATH
        allow(FileReadAll, path_items),
        allow(
            FileReadAll,
            vec![
                // System directories
                Literal("/".into()), // <-- This should be Literal, not Prefix
                Literal("/dev/autofs_nowait".into()),
                Literal("/dev/urandom".into()),
                Literal("/dev/random".into()),
                Literal("/dev/null".into()),
                Literal("/dev/tty".into()),
                Literal("/dev/dtracehelper".into()),
                Prefix("/private/etc/".into()),
                Prefix("/private/var/db/timezone/".into()),
                Prefix("/Applications/Xcode.app/Contents/Developer".into()),
                Prefix("/usr/lib/".into()),
                Prefix("/usr/lib/info/".into()),
                Prefix("/private/var/db/dyld/".into()),
                Prefix("/System/Library/Frameworks/".into()),
                Prefix("/System/Library/PrivateFrameworks/".into()),
                Prefix("/System/Library/".into()),
                Prefix("/System/Volumes/Preboot/Cryptexes/OS".into()),
                Prefix("/System/Cryptexes/OS/".into()),
                Prefix("/Library/Preferences/".into()),
                Regex("/.CFUserTextEncoding$".into()),
                Regex("/Cargo.(lock|toml)$".into()),
                Regex("/.cargo/config$".into()),
            ],
        ),
        // Outbound Network Access
        allow(
            NetworkOutbound,
            vec![
                RemoteIp("*:80".into()),
                RemoteIp("*:443".into()),
                RemoteUnixSocket("/private/var/run/mDNSResponder".into()),
            ],
        ),
    ];

    // RO Cargo Workspace directory
    if let Some(workspace_dir) = workspace.to_str().map(|s| s.to_owned()) {
        // Allowing to write Cargo.lock
        rules.push(allow(
            FileWriteAll,
            vec![Literal(format!("{}/Cargo.lock", &workspace_dir))],
        ));
        rules.push(allow(FileReadAll, vec![Prefix(workspace_dir)]));
    }

    // RO Rustup directory
    if let Ok(home_dir) = env::var("HOME") {
        rules.push(allow(
            FileReadAll,
            vec![Prefix(format!("{}/.rustup/", home_dir))],
        ));
        rules.push(deny(
            FileReadMetadata,
            vec![Prefix(format!("{}/.ssh/", home_dir))],
        ));
    }

    // RW Temp directory
    let tmp_dir = env::temp_dir();
    let real_tmp_path = fs::canonicalize(tmp_dir)?;
    if let Some(tmp_dir) = real_tmp_path.to_str() {
        rules.push(allow(FileReadAll, vec![Prefix(tmp_dir.to_owned())]));
        rules.push(allow(FileWriteAll, vec![Prefix(tmp_dir.to_owned())]));
    }

    // RW Sandbox path
    if let Some(sandbox_path) = sandbox.to_str().map(|p| p.to_owned()) {
        rules.push(allow(
            FileWriteAll,
            vec![
                Prefix(format!("{}/cargo", sandbox_path)),
                Prefix(format!("{}/target", sandbox_path)),
            ],
        ));
    }

    Ok(Profile(rules))
}

/// This module describe types that can be used with macOS seatbelt sandboxing mechanism
/// See: https://reverse.put.as/wp-content/uploads/2011/09/Apple-Sandbox-Guide-v1.0.pdf
mod seatbelt {
    use std::fmt;

    pub struct Profile(pub Vec<Rule>);

    impl fmt::Display for Profile {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            writeln!(f, "(version 1)")?;
            for rule in &self.0 {
                writeln!(f, "{}", rule)?;
            }
            Ok(())
        }
    }

    pub struct Rule(pub Action, pub Operation, pub Vec<Filter>);

    impl fmt::Display for Rule {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let Rule(action, op, filters) = &self;
            if filters.is_empty() {
                write!(f, "({} {})", action.as_str(), op.as_str())
            } else {
                writeln!(f, "({} {}", action.as_str(), op.as_str())?;
                for filter in filters {
                    writeln!(f, "  ({})", filter)?;
                }
                write!(f, ")")
            }
        }
    }

    pub enum Action {
        Allow,
        Deny,
    }

    impl Action {
        pub fn as_str(&self) -> &'static str {
            match self {
                Action::Allow => "allow",
                Action::Deny => "deny",
            }
        }
    }

    #[allow(unused)]
    pub enum Operation {
        Default,
        FileAll,
        FileWriteAll,
        FileReadAll,
        FileIoctl,
        IpcPosixShmReadData,
        UserPreferenceRead,
        FileReadMetadata,
        NetworkOutbound,
        MachLookup,
        IpcAll,
        MachAll,
        NetworkAll,
        ProcessAll,
        ProcessFork,
        Signal,
        SysctlAll,
        SysctlRead,
        SystemAll,
    }

    impl Operation {
        pub fn as_str(&self) -> &'static str {
            match self {
                Operation::Default => "default",
                Operation::FileAll => "file*",
                Operation::FileWriteAll => "file-write*",
                Operation::FileReadAll => "file-read*",
                Operation::FileIoctl => "file-ioctl",
                Operation::IpcPosixShmReadData => "ipc-posix-shm-read-data",
                Operation::UserPreferenceRead => "user-preference-read",
                Operation::FileReadMetadata => "file-read-metadata",
                Operation::NetworkOutbound => "network-outbound",
                Operation::MachLookup => "mach-lookup",
                Operation::IpcAll => "ipc*",
                Operation::MachAll => "mach*",
                Operation::NetworkAll => "network*",
                Operation::ProcessAll => "process*",
                Operation::ProcessFork => "process-fork",
                Operation::Signal => "signal",
                Operation::SysctlAll => "sysctl*",
                Operation::SysctlRead => "sysctl-read",
                Operation::SystemAll => "system*",
            }
        }
    }

    pub enum Filter {
        Literal(String),
        Prefix(String),
        Regex(String),
        RemoteIp(String),
        RemoteUnixSocket(String),
    }

    impl fmt::Display for Filter {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Filter::Literal(p) => write!(f, "literal \"{p}\""),
                Filter::Prefix(p) => write!(f, r#"prefix "{p}""#),
                Filter::Regex(r) => write!(f, r#"regex #"{r}""#),
                Filter::RemoteIp(a) => write!(f, r#"remote ip "{a}""#),
                Filter::RemoteUnixSocket(p) => {
                    write!(f, r#"remote unix-socket (path-literal "{p}")"#)
                }
            }
        }
    }

    pub fn deny(op: Operation, filters: impl IntoIterator<Item = Filter>) -> Rule {
        Rule(Action::Deny, op, filters.into_iter().collect())
    }

    pub fn allow(op: Operation, filters: impl IntoIterator<Item = Filter>) -> Rule {
        Rule(Action::Allow, op, filters.into_iter().collect())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use Filter::*;
        use Operation::*;
        use indoc::indoc;

        #[test]
        fn check_file_prefix_filter() {
            let profile = Profile(vec![allow(
                FileAll,
                vec![Prefix("/home".into()), Prefix("/bin".into())],
            )]);

            let expected_str = indoc! {r#"
                (version 1)
                (allow file*
                  (prefix "/home")
                  (prefix "/bin")
                )
            "#};
            assert_eq!(profile.to_string(), expected_str);
        }

        #[test]
        fn check_network_outbound() {
            let profile = Profile(vec![deny(
                NetworkOutbound,
                vec![RemoteIp("192.168.1.1".into())],
            )]);

            let expected_str = indoc! {r#"
                (version 1)
                (deny network-outbound
                  (remote ip "192.168.1.1")
                )
            "#};
            assert_eq!(profile.to_string(), expected_str);
        }

        #[test]
        fn check_multiple_rules() {
            let profile = Profile(vec![
                allow(FileReadMetadata, vec![Prefix("/etc".into())]),
                deny(ProcessFork, None),
            ]);

            let expected_str = indoc! {r#"
                (version 1)
                (allow file-read-metadata
                  (prefix "/etc")
                )
                (deny process-fork)
            "#};
            assert_eq!(profile.to_string(), expected_str);
        }

        #[test]
        fn check_complex_filters() {
            let profile = Profile(vec![allow(
                IpcAll,
                vec![
                    Regex(".*\\.shm".into()),
                    RemoteUnixSocket("/var/run/socket".into()),
                ],
            )]);

            let expected_str = indoc! {r#"
                (version 1)
                (allow ipc*
                  (regex #".*\.shm")
                  (remote unix-socket (path-literal "/var/run/socket"))
                )
            "#};
            assert_eq!(profile.to_string(), expected_str);
        }
    }
}
