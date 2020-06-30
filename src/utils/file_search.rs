use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::PathBuf;

use console::style;
use failure::Error;
use ignore::overrides::OverrideBuilder;
use ignore::types::TypesBuilder;
use ignore::WalkBuilder;
use log::info;

use crate::utils::progress::{ProgressBar, ProgressStyle};

pub struct ReleaseFileSearch {
    path: PathBuf,
    extensions: BTreeSet<String>,
    ignores: BTreeSet<String>,
    ignore_file: Option<String>,
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
        }
    }

    pub fn extension<E>(&mut self, extension: E) -> &mut Self
    where
        E: Into<String>,
    {
        self.extensions.insert(extension.into());
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

    pub fn ignore<I>(&mut self, ignore: I) -> &mut Self
    where
        I: Into<String>,
    {
        self.ignores.insert(ignore.into());
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

    pub fn collect_file(path: PathBuf) -> Result<ReleaseFileMatch, Error> {
        let mut f = fs::File::open(path.clone())?;
        let mut contents = Vec::new();
        f.read_to_end(&mut contents)?;
        Ok(ReleaseFileMatch {
            base_path: path.clone(),
            path,
            contents,
        })
    }

    pub fn collect_files(&self) -> Result<Vec<ReleaseFileMatch>, Error> {
        let progress_style = ProgressStyle::default_spinner().template(
            "{spinner} Searching for release files...\
        \n  found {prefix:.yellow} {msg:.dim}",
        );

        let progress = ProgressBar::new_spinner();
        progress.enable_steady_tick(100);
        progress.set_style(progress_style);

        let mut collected = Vec::new();

        let mut builder = WalkBuilder::new(&self.path);
        builder.git_exclude(false).git_ignore(false).ignore(false);

        if !&self.extensions.is_empty() {
            let mut types_builder = TypesBuilder::new();
            for ext in &self.extensions {
                let ext_name = ext.replace('.', "__");
                types_builder.add(&ext_name, &format!("*.{}", ext))?;
            }
            builder.types(types_builder.select("all").build()?);
        }

        if let Some(ignore_file) = &self.ignore_file {
            // This could yield an optional partial error
            // We ignore this error to match behavior of git
            builder.add_ignore(ignore_file);
        }

        if !&self.ignores.is_empty() {
            let mut override_builder = OverrideBuilder::new(&self.path);
            for ignore in &self.ignores {
                override_builder.add(&ignore)?;
            }
            builder.overrides(override_builder.build()?);
        }

        for result in builder.build() {
            let file = result?;
            if file.file_type().map_or(false, |t| t.is_dir()) {
                continue;
            }
            progress.set_message(&format!("{}", file.path().display()));

            info!(
                "found: {} ({} bytes)",
                file.path().display(),
                file.metadata().unwrap().len()
            );

            let mut f = fs::File::open(file.path())?;
            let mut contents = Vec::new();
            f.read_to_end(&mut contents)?;

            let file_match = ReleaseFileMatch {
                base_path: self.path.clone(),
                path: file.path().to_path_buf(),
                contents,
            };
            collected.push(file_match);

            progress.set_prefix(&collected.len().to_string());
        }

        progress.finish_and_clear();
        println!(
            "{} Found {} release {}",
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
