use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Read};
use std::path::Path;

use failure::Error;
use serde::Serialize;
use zip::{write::FileOptions, ZipWriter};

static MANIFEST_PATH: &str = "manifest.json";

fn trim_end_matches<F>(string: &mut String, pat: F)
where
    F: FnMut(char) -> bool,
{
    let cutoff = string.trim_end_matches(pat).len();
    string.truncate(cutoff);
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    #[serde(rename = "source")]
    Script,
    #[serde(rename = "minified_source")]
    MinifiedScript,
    SourceMap,
    IndexedRamBundle,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ArtifactInfo {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub url: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub ty: Option<ArtifactType>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
}

impl ArtifactInfo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.url.is_empty() && self.ty.is_none()
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ArtifactManifest {
    pub org: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    pub release: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dist: Option<String>,
    pub files: HashMap<String, ArtifactInfo>,
}

impl ArtifactManifest {
    pub fn new() -> Self {
        Self::default()
    }
}

pub struct ArtifactBundleWriter {
    manifest: ArtifactManifest,
    writer: ZipWriter<BufWriter<File>>,
}

impl ArtifactBundleWriter {
    pub fn new(file: File) -> Self {
        ArtifactBundleWriter {
            manifest: ArtifactManifest::new(),
            writer: ZipWriter::new(BufWriter::new(file)),
        }
    }

    pub fn create<P>(path: P) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        Ok(Self::new(file))
    }

    pub fn set_org<S>(&mut self, org: S)
    where
        S: Into<String>,
    {
        self.manifest.org = org.into();
    }

    pub fn set_project<S>(&mut self, project: Option<S>)
    where
        S: Into<String>,
    {
        self.manifest.project = project.map(Into::into);
    }

    pub fn set_release<S>(&mut self, release: S)
    where
        S: Into<String>,
    {
        self.manifest.release = release.into();
    }

    pub fn set_dist<S>(&mut self, dist: Option<S>)
    where
        S: Into<String>,
    {
        self.manifest.dist = dist.map(Into::into);
    }

    pub fn has_file<S>(&self, path: S) -> bool
    where
        S: AsRef<str>,
    {
        self.manifest.files.contains_key(path.as_ref())
    }

    pub fn add_file<S, R>(&mut self, path: S, mut file: R, info: ArtifactInfo) -> Result<(), Error>
    where
        S: AsRef<str>,
        R: Read,
    {
        let path = format!("files/{}", path.as_ref());
        let unique_path = self.unique_path(path);

        self.writer
            .start_file(unique_path.clone(), FileOptions::default())?;
        std::io::copy(&mut file, &mut self.writer)?;

        self.manifest.files.insert(unique_path, info);
        Ok(())
    }

    pub fn finish(mut self) -> Result<(), Error> {
        self.write_manifest()?;
        self.writer.finish()?;
        Ok(())
    }

    fn unique_path(&self, mut path: String) -> String {
        let mut duplicates = 0;

        while self.has_file(&path) {
            duplicates += 1;
            match duplicates {
                1 => path.push_str(".1"),
                _ => {
                    trim_end_matches(&mut path, char::is_numeric);
                    write!(path, ".{}", duplicates).unwrap();
                }
            }
        }

        path
    }

    fn write_manifest(&mut self) -> Result<(), Error> {
        self.writer
            .start_file(MANIFEST_PATH, FileOptions::default())?;
        serde_json::to_writer(&mut self.writer, &self.manifest)?;
        Ok(())
    }
}
