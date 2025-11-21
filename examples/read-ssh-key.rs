use std::{env, fs, io};

fn main() -> Result<(), io::Error> {
    // This example tries to read first file from .ssh directory
    let ssh_path = env::home_dir().unwrap().join(".ssh");

    for entry in fs::read_dir(&ssh_path)? {
        let entry = entry?;
        if entry.metadata()?.is_file() {
            fs::read_to_string(entry.path())?;
        }
    }
    Ok(())
}
