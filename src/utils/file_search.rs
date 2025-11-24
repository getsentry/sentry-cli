use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io::Read as _;
use std::path::PathBuf;

use anyhow::Result;
use console::style;
use glob::Pattern;
use ignore::types::TypesBuilder;
use ignore::WalkBuilder;
use log::{info, warn};

use crate::utils::progress::{ProgressBar, ProgressStyle};

use super::fs::{decompress_gzip_content, is_gzip_compressed};

pub struct ReleaseFileSearch {
    path: PathBuf,
    extensions: BTreeSet<String>,
    ignores: BTreeSet<String>,
    ignore_file: Option<String>,
    decompress: bool,
}

#[derive(Eq, PartialEq, Hash)]
pub struct ReleaseFileMatch {
    pub base_path: PathBuf,
    pub path: PathBuf,
    pub contents: Vec<u8>,
}

impl ReleaseFileSearch {
    pub fn new(path: PathBuf) -> Self {
        ReleaseFileSearch {
            path,
            extensions: BTreeSet::new(),
            ignore_file: None,
            ignores: BTreeSet::new(),
            decompress: false,
        }
    }

    pub fn decompress(&mut self, decompress: bool) -> &mut Self {
        self.decompress = decompress;
        self
    }

    pub fn extensions<E>(&mut self, extensions: E) -> &mut Self
    where
        E: IntoIterator,
        E::Item: Into<String>,
    {
        for extension in extensions {
            self.extensions.insert(extension.into());
        }
        self
    }

    pub fn ignores<I>(&mut self, ignores: I) -> &mut Self
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        for ignore in ignores {
            self.ignores.insert(ignore.into());
        }
        self
    }

    pub fn ignore_file<P>(&mut self, path: P) -> &mut Self
    where
        P: Into<String>,
    {
        let path = path.into();
        if !path.is_empty() {
            self.ignore_file = Some(path);
        }
        self
    }

    pub fn collect_file(path: PathBuf) -> Result<ReleaseFileMatch> {
        // NOTE: `collect_file` currently do not handle gzip decompression,
        // as its mostly used for 3rd tools like xcode, appcenter or gradle.
        let mut f = fs::File::open(path.clone())?;
        let mut contents = Vec::new();
        f.read_to_end(&mut contents)?;

        Ok(ReleaseFileMatch {
            base_path: path.clone(),
            path,
            contents,
        })
    }

    pub fn collect_files(&self) -> Result<Vec<ReleaseFileMatch>> {
        let progress_style = ProgressStyle::default_spinner().template(
            "{spinner} Searching for files...\
        \n  found {prefix:.yellow} {msg:.dim}",
        );

        let pb = ProgressBar::new_spinner();
        pb.enable_steady_tick(100);
        pb.set_style(progress_style);

        let mut collected = Vec::new();

        let mut builder = WalkBuilder::new(&self.path);
        builder
            .follow_links(true)
            .git_exclude(false)
            .git_ignore(false)
            .ignore(false);

        if !&self.extensions.is_empty() {
            let mut types_builder = TypesBuilder::new();
            for ext in &self.extensions {
                let ext_name = ext.replace('.', "");
                types_builder.add(&ext_name, &format!("*.{ext}"))?;
            }
            builder.types(types_builder.select("all").build()?);
        }

        if let Some(ignore_file) = &self.ignore_file {
            // This could yield an optional partial error
            // We ignore this error to match behavior of git
            builder.add_ignore(ignore_file);
        }

        // Compile ignore patterns relative to CWD (not relative to search path)
        let cwd = env::current_dir()?;
        let ignore_patterns: Vec<Pattern> = self
            .ignores
            .iter()
            .filter_map(|pattern| {
                // Patterns starting with ! are negations (handled in upload.rs/inject.rs)
                // Remove the ! prefix for glob pattern compilation
                let pattern_str = pattern.strip_prefix('!').unwrap_or(pattern);
                match Pattern::new(pattern_str) {
                    Ok(p) => Some(p),
                    Err(e) => {
                        warn!("Invalid ignore pattern '{}': {}", pattern_str, e);
                        None
                    }
                }
            })
            .collect();

        // Use filter_entry to match patterns relative to CWD
        if !ignore_patterns.is_empty() {
            builder.filter_entry(move |entry| {
                let entry_path = entry.path();

                // Try to make the path relative to CWD, or use absolute path if that fails
                let check_path = entry_path.strip_prefix(&cwd).unwrap_or(entry_path);

                // Check if any pattern matches - if so, ignore (return false)
                for pattern in &ignore_patterns {
                    if pattern.matches_path(check_path) {
                        return false;
                    }
                }

                // No patterns matched, keep this entry
                true
            });
        }

        for result in builder.build() {
            let file = result?;
            if file.file_type().is_some_and(|t| t.is_dir()) {
                continue;
            }
            pb.set_message(format!("{}", file.path().display()));

            info!("found: {} ({} bytes)", file.path().display(), {
                #[expect(clippy::unwrap_used, reason = "legacy code")]
                file.metadata().unwrap().len()
            });

            let mut f = fs::File::open(file.path())?;
            let mut contents = Vec::new();
            f.read_to_end(&mut contents)?;

            if self.decompress && is_gzip_compressed(&contents) {
                contents = decompress_gzip_content(&contents).unwrap_or_else(|_| {
                    warn!("Could not decompress: {}", file.path().display());
                    contents
                });
            }

            let file_match = ReleaseFileMatch {
                base_path: self.path.clone(),
                path: file.path().to_path_buf(),
                contents,
            };
            collected.push(file_match);

            pb.set_prefix(collected.len().to_string());
        }

        pb.finish_and_clear();
        println!(
            "{} Found {} {}",
            style(">").dim(),
            style(collected.len()).yellow(),
            match collected.len() {
                1 => "file",
                _ => "files",
            }
        );

        Ok(collected)
    }
}
