use std::borrow::Borrow;
use std::io::BufWriter;

use anyhow::Result;
use indicatif::ProgressStyle;
use sentry::types::DebugId;
use symbolic::debuginfo::sourcebundle::{
    SourceBundleErrorKind, SourceBundleWriter, SourceFileInfo,
};
use url::Url;

use crate::utils::file_upload::{SourceFile, UploadContext};
use crate::utils::fs::TempFile;
use crate::utils::non_empty::NonEmptySlice;
use crate::utils::progress::ProgressBar;

#[derive(Clone, Copy, Debug, Default)]
pub struct BundleContext<'a> {
    org: &'a str,
    projects: Option<NonEmptySlice<'a, String>>,
    note: Option<&'a str>,
    release: Option<&'a str>,
    dist: Option<&'a str>,
}

impl<'a> From<&'a UploadContext<'a>> for BundleContext<'a> {
    fn from(context: &'a UploadContext<'a>) -> Self {
        Self {
            org: context.org,
            projects: context.projects,
            note: context.note,
            release: context.release,
            dist: context.dist,
        }
    }
}

/// Builds a source bundle from a list of source files, setting the metadata
/// from the upload context.
///
/// Returns a `TempFile` containing the source bundle.
pub fn build<'a, C, F, S>(context: C, files: F, debug_id: Option<DebugId>) -> Result<TempFile>
where
    C: Into<BundleContext<'a>>,
    F: IntoIterator<Item = S>,
    S: Borrow<SourceFile>,
{
    let context = context.into();
    let files = files.into_iter().collect::<Vec<_>>();

    let progress_style = ProgressStyle::default_bar().template(
        "{prefix:.dim} Bundling files for upload... {msg:.dim}\
       \n{wide_bar}  {pos}/{len}",
    );

    let pb = ProgressBar::new(files.len());
    pb.set_style(progress_style);
    pb.set_prefix(">");

    let archive = TempFile::create()?;
    let mut bundle = SourceBundleWriter::start(BufWriter::new(archive.open()?))?;

    // source bundles get a random UUID as debug id
    let debug_id = debug_id.unwrap_or_else(|| build_debug_id(&files));
    bundle.set_attribute("debug_id", debug_id.to_string());

    if let Some(note) = context.note {
        bundle.set_attribute("note", note.to_owned());
    }

    bundle.set_attribute("org".to_owned(), context.org.to_owned());
    if let Some([project]) = context.projects.as_deref() {
        // Only set project if there is exactly one project
        bundle.set_attribute("project".to_owned(), project);
    }
    if let Some(release) = context.release {
        bundle.set_attribute("release".to_owned(), release.to_owned());
    }
    if let Some(dist) = context.dist {
        bundle.set_attribute("dist".to_owned(), dist.to_owned());
    }

    let mut bundle_file_count = 0;

    for file in files.iter().map(Borrow::borrow) {
        pb.inc(1);
        pb.set_message(&file.url);

        let mut info = SourceFileInfo::new();
        info.set_ty(file.ty);
        info.set_url(file.url.clone());
        for (k, v) in &file.headers {
            info.add_header(k.clone(), v.clone());
        }

        let bundle_path = url_to_bundle_path(&file.url)?;
        if let Err(e) = bundle.add_file(bundle_path, file.contents.as_slice(), info) {
            if e.kind() == SourceBundleErrorKind::ReadFailed {
                log::info!(
                    "Skipping {} because it is not valid UTF-8.",
                    file.path.display()
                );
                continue;
            } else {
                return Err(e.into());
            }
        }
        bundle_file_count += 1;
    }

    bundle.finish()?;

    pb.finish_with_duration("Bundling");

    println!(
        "{} Bundled {} {} for upload",
        console::style(">").dim(),
        console::style(bundle_file_count).yellow(),
        match bundle_file_count {
            1 => "file",
            _ => "files",
        }
    );

    println!(
        "{} Bundle ID: {}",
        console::style(">").dim(),
        console::style(debug_id).yellow(),
    );

    Ok(archive)
}

/// Creates a debug id from a map of source files by hashing each file's
/// URL, contents, type, and headers.
fn build_debug_id<S>(files: &[S]) -> DebugId
where
    S: Borrow<SourceFile>,
{
    let mut hash = sha1_smol::Sha1::new();
    for source_file in files.iter().map(Borrow::borrow) {
        hash.update(source_file.url.as_bytes());
        hash.update(&source_file.contents);
        hash.update(format!("{:?}", source_file.ty).as_bytes());

        for (key, value) in &source_file.headers {
            hash.update(key.as_bytes());
            hash.update(value.as_bytes());
        }
    }

    let mut sha1_bytes = [0u8; 16];
    sha1_bytes.copy_from_slice(&hash.digest().bytes()[..16]);
    DebugId::from_uuid(uuid::Builder::from_sha1_bytes(sha1_bytes).into_uuid())
}

fn url_to_bundle_path(url: &str) -> Result<String> {
    let base = Url::parse("http://~").expect("this url is valid");
    let url = if let Some(rest) = url.strip_prefix("~/") {
        base.join(rest)?
    } else {
        base.join(url)?
    };

    let mut path = url.path().to_owned();
    if let Some(fragment) = url.fragment() {
        path = format!("{path}#{fragment}");
    }
    if path.starts_with('/') {
        path.remove(0);
    }

    Ok(match url.host_str() {
        Some("~") => format!("_/_/{path}"),
        Some(host) => format!("{}/{host}/{path}", url.scheme()),
        None => format!("{}/_/{path}", url.scheme()),
    })
}

#[cfg(test)]
mod tests {
    use sha1_smol::Sha1;
    use symbolic::debuginfo::sourcebundle::SourceFileType;

    use crate::utils::file_upload::SourceFile;

    use super::*;

    #[test]
    fn test_url_to_bundle_path() {
        assert_eq!(url_to_bundle_path("~/bar").unwrap(), "_/_/bar");
        assert_eq!(url_to_bundle_path("~/foo/bar").unwrap(), "_/_/foo/bar");
        assert_eq!(
            url_to_bundle_path("~/dist/js/bundle.js.map").unwrap(),
            "_/_/dist/js/bundle.js.map"
        );
        assert_eq!(
            url_to_bundle_path("~/babel.config.js").unwrap(),
            "_/_/babel.config.js"
        );

        assert_eq!(url_to_bundle_path("~/#/bar").unwrap(), "_/_/#/bar");
        assert_eq!(url_to_bundle_path("~/foo/#/bar").unwrap(), "_/_/foo/#/bar");
        assert_eq!(
            url_to_bundle_path("~/dist/#js/bundle.js.map").unwrap(),
            "_/_/dist/#js/bundle.js.map"
        );
        assert_eq!(
            url_to_bundle_path("~/#foo/babel.config.js").unwrap(),
            "_/_/#foo/babel.config.js"
        );
    }

    #[test]
    fn build_deterministic() {
        let projects_slice = &["wat-project".into()];
        let context = BundleContext {
            org: "wat-org",
            projects: Some(projects_slice.into()),
            release: None,
            dist: None,
            note: None,
        };

        let source_files = ["bundle.min.js.map", "vendor.min.js.map"]
            .into_iter()
            .map(|name| SourceFile {
                url: format!("~/{name}"),
                path: format!("tests/integration/_fixtures/{name}").into(),
                contents: std::fs::read(format!("tests/integration/_fixtures/{name}"))
                    .unwrap()
                    .into(),
                ty: SourceFileType::SourceMap,
                headers: Default::default(),
                messages: Default::default(),
                already_uploaded: false,
            })
            .collect::<Vec<_>>();

        let file = build(context, &source_files, None).unwrap();

        let buf = std::fs::read(file.path()).unwrap();
        let hash = Sha1::from(buf);
        assert_eq!(
            hash.digest().to_string(),
            "f0e25ae149b711c510148e022ebc883ad62c7c4c"
        );
    }
}
