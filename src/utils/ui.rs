use std::io;
use std::io::Write as _;

use crate::utils::progress::{ProgressBar, ProgressStyle};

/// Prints a message and loops until yes or no is entered.
pub fn prompt_to_continue(message: &str) -> io::Result<bool> {
    loop {
        print!("{message} [y/n] ");
        io::stdout().flush()?;

        let mut buf = String::new();
        io::stdin().read_line(&mut buf)?;
        let input = buf.trim().to_lowercase();

        if input == "y" {
            return Ok(true);
        } else if input == "n" {
            return Ok(false);
        }
        println!("invalid input!");
    }
}

/// Prompts for input and returns it.
pub fn prompt(message: &str) -> io::Result<String> {
    loop {
        print!("{message}: ");
        io::stdout().flush()?;
        let mut buf = String::new();
        io::stdin().read_line(&mut buf)?;
        let input = buf.trim();
        if !input.is_empty() {
            return Ok(input.to_owned());
        }
    }
}

/// Capitalizes a string and returns it.
pub fn capitalize_string(s: &str) -> String {
    let mut bytes = s.as_bytes().to_vec();
    bytes.make_ascii_lowercase();
    bytes[0] = bytes[0].to_ascii_uppercase();
    #[expect(clippy::unwrap_used, reason = "legacy code")]
    String::from_utf8(bytes).unwrap()
}

/// Creates a progress bar for byte stuff
pub fn make_byte_progress_bar(length: u64) -> ProgressBar {
    let pb = ProgressBar::new(length as usize);
    pb.set_style(
        ProgressStyle::default_bar().template("{wide_bar}  {bytes}/{total_bytes} ({eta})"),
    );
    pb
}
