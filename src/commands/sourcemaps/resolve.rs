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

fn print_token(token: &Token<'_>) {
    if let Some(name) = token.get_name() {
        println!("  name: {name:?}");
    } else {
        println!("  name: not found");
    }
    if let Some(source) = token.get_source() {
        println!("  source file: {source:?}");
    } else {
        println!("  source file: not found");
    }
    println!("  source line: {}", token.get_src_line());
    println!("  source column: {}", token.get_src_col());
    println!("  minified line: {}", token.get_dst_line());
    println!("  minified column: {}", token.get_dst_col());
    if let Some(view) = token.get_source_view() {
        println!("  source code:");
        print_source(token, view);
    } else if token.get_source_view().is_none() {
        println!("  cannot find source");
    } else {
        println!("  cannot find source line");
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

    // perform a lookup
    if let Some((line, column)) = lookup_pos(matches) {
        println!("lookup line: {line}, column: {column}:");
        if let Some(token) = sm.lookup_token(line, column) {
            print_token(&token);
        } else {
            println!("  - no match");
        }
    }

    Ok(())
}
