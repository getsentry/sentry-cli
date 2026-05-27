use std::io::{self, IsTerminal as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context as _, Result};
use log::info;

const MIN_SUPPORTED_VERSION: &str = "4.0.0";

fn check_version(binary_path: &Path) {
    let output = Command::new(binary_path).arg("--version").output();

    match output {
        Ok(out) if out.status.success() => {
            let version_str = String::from_utf8_lossy(&out.stdout).trim().to_owned();
            let version = version_str
                .strip_prefix("odiff ")
                .unwrap_or(&version_str)
                .trim();

            let min = semver::Version::parse(MIN_SUPPORTED_VERSION)
                .expect("MIN_SUPPORTED_VERSION is valid semver");
            if let Ok(installed) = semver::Version::parse(version) {
                if installed < min {
                    eprintln!(
                        "Warning: odiff {version} is below the minimum supported version \
                         ({MIN_SUPPORTED_VERSION}). You may experience issues."
                    );
                }
            }
        }
        _ => {
            eprintln!("Warning: Could not determine odiff version. You may experience issues.");
        }
    }
}

fn prompt_npm_install() -> Result<PathBuf> {
    if !io::stdin().is_terminal() || !io::stderr().is_terminal() {
        bail!(
            "This command requires `odiff`, but it is not installed.\n\n\
             `odiff` can be installed with npm:\n\n    \
             npm install -g odiff-bin"
        );
    }

    eprintln!("This command requires `odiff`, but it is not installed.");
    eprintln!();
    eprintln!("`odiff` can be installed with npm:");
    eprintln!();
    eprintln!("    npm install -g odiff-bin");
    eprintln!();
    eprint!("Would you like us to install `odiff` with npm for you? [y/N] ");
    io::stderr().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let trimmed_input = input.trim();
    if !trimmed_input.eq_ignore_ascii_case("y") && !trimmed_input.eq_ignore_ascii_case("yes") {
        bail!(
            "`odiff` is required but not installed. \
             Install it with: npm install -g odiff-bin"
        );
    }

    eprintln!();
    eprintln!("Running `npm install -g odiff-bin`...");
    eprintln!();

    let status = Command::new("npm")
        .args(["install", "-g", "odiff-bin"])
        .status()
        .context("Failed to run npm. Is npm installed?")?;

    if !status.success() {
        bail!("npm install failed. Please install odiff manually: npm install -g odiff-bin");
    }

    match which::which("odiff") {
        Ok(path) => Ok(path),
        Err(_) => {
            let hint = Command::new("npm")
                .args(["prefix", "-g"])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| {
                    let prefix = String::from_utf8_lossy(&o.stdout).trim().to_owned();
                    format!(" npm global prefix: {prefix}")
                })
                .unwrap_or_default();

            bail!(
                "odiff was installed but could not be found on PATH.{hint}\n\
                 You may need to restart your shell."
            );
        }
    }
}

pub fn ensure_binary() -> Result<PathBuf> {
    if let Ok(system_path) = which::which("odiff") {
        info!("Using system odiff at {}", system_path.display());
        check_version(&system_path);
        return Ok(system_path);
    }

    prompt_npm_install()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_min_version_parses() {
        semver::Version::parse(MIN_SUPPORTED_VERSION)
            .expect("MIN_SUPPORTED_VERSION should be valid semver");
    }
}
