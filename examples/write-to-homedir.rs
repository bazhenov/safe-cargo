use std::{env, fs, io};

fn main() -> Result<(), io::Error> {
    // This example tries to read config file in a home dir
    let home = env::home_dir().unwrap();

    fs::write(home.join(".bashrc-example"), r#"echo "Pwned""#)
}
