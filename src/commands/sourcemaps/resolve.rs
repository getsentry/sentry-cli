use std::cmp;
use std::fs;
use std::path::PathBuf;

use anyhow::{format_err, Result};
use clap::{Arg, ArgMatches, Command};
use sourcemap::{DecodedMap, SourceView, Token};

pub fn make_command(command: Command) -> Command {
    command
        .about("Resolve sourcemap for a given line/column position.")
        .arg(
            Arg::new("path")
                .value_name("PATH")
                .help("The sourcemap to resolve."),
        )
        .arg(
            Arg::new("line")
                .long("line")
                .short('l')
                .value_name("LINE")
                .value_parser(clap::value_parser!(u32))
                .help("Line number for minified source."),
        )
        .arg(
            Arg::new("column")
                .long("column")
                .short('c')
                .value_name("COLUMN")
                .value_parser(clap::value_parser!(u32))
                .help("Column number for minified source."),
        )
}

/// Returns the zero indexed position from matches
fn lookup_pos(matches: &ArgMatches) -> Option<(u32, u32)> {
    Some((
        matches.get_one::<u32>("line").map_or(0, |x| x - 1),
        matches.get_one::<u32>("column").map_or(0, |x| x - 1),
    ))
}

fn count_whitespace_prefix(test: &str) -> i32 {
    let mut result = 0;
    for c in test.chars() {
        if c.is_whitespace() {
            result += 1;
        } else {
            return result;
        }
    }
    result
}

pub fn print_source(token: &Token<'_>, view: &SourceView) {
    let mut lines: Vec<&str> = vec![];
    for offset in &[-3, -2, -1, 0, 1, 2, 3] {
        let line = token.get_src_line() as isize + offset;
        if line < 0 {
            continue;
        }
        if let Some(line) = view.get_line(line as u32) {
            lines.push(line);
        }
    }
    let lowest_indent = lines
        .iter()
        .map(|l| count_whitespace_prefix(l))
        .min()
        .unwrap_or(0);

    for line in lines {
        println!("    {}", &line[(lowest_indent as usize)..]);
    }
}

fn dst_location(token: &Token) -> (u32, u32) {
    (token.get_dst_line() + 1, token.get_dst_col() + 1)
}

fn src_location(token: &Token) -> (u32, u32) {
    (token.get_src_line() + 1, token.get_src_col() + 1)
}

fn as_string_len<T: ToString>(to_string: T) -> usize {
    to_string.to_string().len()
}

fn print_token(token: &Token<'_>) {
    let token_display_name = match token.get_name() {
        Some(name) => format!("token \"{name}\""),
        None => String::from("token (unnamed)"),
    };
    let source_file = match token.get_source() {
        Some(file) => format!("{file:>4}"),
        None => String::from("(unknown path)"),
    };

    let (dst_line, dst_col) = dst_location(token);
    let [dst_line_digits, dst_col_digits] = [dst_line, dst_col].map(as_string_len);

    let (src_line, src_col) = src_location(token);
    let [src_line_digits, src_col_digits] = [src_line, src_col].map(as_string_len);

    let line_align = cmp::max(dst_line_digits, src_line_digits);
    let col_align = cmp::max(dst_col_digits, src_col_digits);

    let output_minified_line = format!(
        "Found the nearest {token_display_name} at line {:>line_align$}, column {:>col_align$} in the minified file.",
        dst_line,
        dst_col,
    );

    let output_source_line = format!(
        "- The same token is located at line {:>line_align$}, column {:>col_align$} in source file {}.",
        src_line,
        src_col,
        source_file,
    );

    let output_minified_line_align = 2 + output_minified_line.len();
    let output_source_line_align = output_minified_line.len() + source_file.len() - 3;

    println!("{output_minified_line:>output_minified_line_align$}");
    println!();
    println!("{output_source_line:>output_source_line_align$}");
    println!("\n");

    if let Some(view) = token.get_source_view() {
        println!("  Source code:");
        print_source(token, view);
    } else if token.get_source_view().is_none() {
        println!("  Cannot find source");
    } else {
        println!("  Cannot find source line");
    }
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let sourcemap_path = matches
        .get_one::<String>("path")
        .ok_or_else(|| format_err!("Sourcemap not provided"))?;

    let sm = sourcemap::decode_slice(&fs::read(PathBuf::from(sourcemap_path))?)?;

    let ty = match sm {
        DecodedMap::Regular(..) => "regular",
        DecodedMap::Index(..) => "indexed",
        DecodedMap::Hermes(..) => "hermes",
    };
    println!("source map path: {sourcemap_path:?}");
    println!("source map type: {ty}");
    println!();

    // perform a lookup
    if let Some((line, column)) = lookup_pos(matches) {
        println!(
            "Searching for token nearest to line {}, column {} in the minified file:\n",
            line + 1,
            column + 1
        );
        if let Some(token) = sm.lookup_token(line, column) {
            print_token(&token);
        } else {
            println!("  - no token found!");
        }
    }

    Ok(())
}
