use std::io;
use std::io::{Read, Write};

use crate::utils::progress::{ProgressBar, ProgressStyle};

/// Prints a message and loops until yes or no is entered.
pub fn prompt_to_continue(message: &str) -> io::Result<bool> {
    loop {
        print!("{message} [y/n] ");
        io::stdout().flush()?;

        let mut buf = String::new();
        io::stdin().read_line(&mut buf)?;
        let input = buf.trim();

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
    String::from_utf8(bytes).unwrap()
}

/// Like ``io::copy`` but advances a progress bar set to bytes.
pub fn copy_with_progress<R: ?Sized, W: ?Sized>(
    pb: &ProgressBar,
    reader: &mut R,
    writer: &mut W,
) -> io::Result<u64>
where
    R: Read,
    W: Write,
{
    let mut buf = [0; 16384];
    let mut written = 0;
    loop {
        let len = match reader.read(&mut buf) {
            Ok(0) => return Ok(written),
            Ok(len) => len,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        writer.write_all(&buf[..len])?;
        written += len as u64;
        pb.inc(len as u64);
    }
}

/// Creates a progress bar for byte stuff
pub fn make_byte_progress_bar(length: u64) -> ProgressBar {
    let pb = ProgressBar::new(length as usize);
    pb.set_style(
        ProgressStyle::default_bar().template("{wide_bar}  {bytes}/{total_bytes} ({eta})"),
    );
    pb
}
