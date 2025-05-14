use crate::commands::init::KittyError;
use std::fs;
use std::{path::Path, process::Command};

fn run_with_sudo(command: &[&str]) -> Result<(), KittyError> {
    let status = Command::new("sudo")
        .args(command)
        .status()
        .map_err(|e| KittyError::Io(e))?;

    if !status.success() {
        return Err(KittyError::Io(io::Error::new(
            io::ErrorKind::Other,
            "Command execution failed",
        )));
    }

    Ok(())
}

fn copy_file_with_privileges(source: &Path, dest: &Path) -> Result<(), KittyError> {
    // First try to copy directly
    let copy_result = fs::copy(source, dest);

    if let Err(e) = copy_result {
        if e.kind() == io::ErrorKind::PermissionDenied {
            // Permission denied, try with sudo
            println!("Permission denied, attempting with elevated privileges...");
            run_with_sudo(&["cp", source.to_str().unwrap(), dest.to_str().unwrap()])
        } else {
            Err(KittyError::Io(e))
        }
    } else {
        Ok(())
    }
}
