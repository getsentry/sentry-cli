use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use clap::{Arg, ArgMatches, Command};
use serde::Serialize;
use walkdir::WalkDir;

use crate::utils::fs::{path_as_url, IMAGE_EXTENSIONS};
use crate::utils::odiff::binary::ensure_binary;
use crate::utils::odiff::server::{OdiffOptions, OdiffResponse, OdiffServer};

#[derive(Serialize)]
struct DiffReport {
    base_dir: String,
    head_dir: String,
    output_dir: String,
    threshold: f64,
    summary: DiffSummary,
    images: Vec<ImageResult>,
}

#[derive(Serialize)]
struct DiffSummary {
    total: usize,
    changed: usize,
    unchanged: usize,
    added: usize,
    removed: usize,
    skipped: usize,
    errored: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum ImageStatus {
    Unchanged,
    Changed,
    LayoutChanged,
    Added,
    Removed,
    Skipped,
    Error,
}

#[derive(Serialize)]
struct ImageResult {
    name: String,
    status: ImageStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    diff_percentage: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diff_pixel_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diff_mask_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl ImageResult {
    fn simple(name: String, status: ImageStatus) -> Self {
        Self {
            name,
            status,
            diff_percentage: None,
            diff_pixel_count: None,
            diff_mask_path: None,
            error: None,
        }
    }

    fn error(name: String, err: String) -> Self {
        Self {
            name,
            status: ImageStatus::Error,
            diff_percentage: None,
            diff_pixel_count: None,
            diff_mask_path: None,
            error: Some(err),
        }
    }

    fn from_response(
        name: String,
        status: ImageStatus,
        response: &OdiffResponse,
        mask: Option<String>,
    ) -> Self {
        Self {
            name,
            status,
            diff_percentage: response.diff_percentage,
            diff_pixel_count: response.diff_count,
            diff_mask_path: mask,
            error: None,
        }
    }
}

struct CategorizedImages {
    matched: BTreeSet<PathBuf>,
    added: BTreeSet<PathBuf>,
    removed: BTreeSet<PathBuf>,
    skipped: BTreeSet<PathBuf>,
}

pub fn make_command(command: Command) -> Command {
    command
        .about("Compare two directories of snapshot images locally using odiff.")
        .long_about("Compare two directories of snapshot images locally using odiff.")
        .arg(
            Arg::new("base_dir")
                .value_name("BASE_DIR")
                .help("Path to baseline image directory.")
                .required(true),
        )
        .arg(
            Arg::new("head_dir")
                .value_name("HEAD_DIR")
                .help("Path to head image directory.")
                .required(true),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .value_name("DIR")
                .help("Directory for diff mask images.")
                .default_value("./diff-output/"),
        )
        .arg(
            Arg::new("threshold")
                .long("threshold")
                .value_name("FLOAT")
                .value_parser(|s: &str| {
                    let v: f64 = s.parse().map_err(|e| format!("invalid float: {e}"))?;
                    if !(0.0..=1.0).contains(&v) {
                        return Err("value must be between 0.0 and 1.0".to_owned());
                    }
                    Ok(v)
                })
                .default_value("0.01")
                .help("Pixel color difference threshold (0.0-1.0)."),
        )
        .arg(
            Arg::new("no_antialiasing")
                .long("no-antialiasing")
                .action(clap::ArgAction::SetTrue)
                .help("Disable antialiasing detection."),
        )
        .arg(
            Arg::new("fail_on_diff")
                .long("fail-on-diff")
                .action(clap::ArgAction::SetTrue)
                .help("Exit with code 1 if any diffs are found."),
        )
        .arg(
            Arg::new("selective")
                .long("selective")
                .action(clap::ArgAction::SetTrue)
                .help("Treat missing base images as skipped instead of removed."),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let base_dir = PathBuf::from(
        matches
            .get_one::<String>("base_dir")
            .expect("base_dir is required"),
    );
    let head_dir = PathBuf::from(
        matches
            .get_one::<String>("head_dir")
            .expect("head_dir is required"),
    );
    let output_dir = PathBuf::from(
        matches
            .get_one::<String>("output")
            .expect("output has a default value"),
    );
    let threshold = *matches
        .get_one::<f64>("threshold")
        .expect("threshold has a default value");
    let antialiasing = !matches.get_flag("no_antialiasing");
    let fail_on_diff = matches.get_flag("fail_on_diff");
    let selective = matches.get_flag("selective");

    if !base_dir.is_dir() {
        bail!("Base directory does not exist: {}", base_dir.display());
    }
    if !head_dir.is_dir() {
        bail!("Head directory does not exist: {}", head_dir.display());
    }

    let base_files = collect_image_files(&base_dir)?;
    let head_files = collect_image_files(&head_dir)?;
    let categorized = categorize_images(&base_files, &head_files, selective);

    let mut images: Vec<ImageResult> = Vec::new();

    if !categorized.matched.is_empty() {
        fs::create_dir_all(&output_dir)?;

        let binary_path = ensure_binary()?;
        let options = OdiffOptions {
            threshold,
            antialiasing,
            output_diff_mask: true,
        };
        let mut server = OdiffServer::start(&binary_path)?;
        let matched_paths: Vec<_> = categorized.matched.iter().collect();

        for (i, rel_path) in matched_paths.iter().enumerate() {
            let base_file = base_dir.join(rel_path);
            let head_file = head_dir.join(rel_path);
            let name = path_as_url(rel_path);

            match files_are_identical(&base_file, &head_file) {
                Ok(true) => {
                    images.push(ImageResult::simple(name, ImageStatus::Unchanged));
                    continue;
                }
                Err(err) => {
                    images.push(ImageResult::error(name, format!("{err:#}")));
                    continue;
                }
                Ok(false) => {}
            }

            let mask_path = output_dir.join(rel_path).with_extension("png");
            if let Some(parent) = mask_path.parent() {
                if let Err(err) = fs::create_dir_all(parent) {
                    images.push(ImageResult::error(name, format!("{err:#}")));
                    continue;
                }
            }

            match server.compare(&base_file, &head_file, &mask_path, &options) {
                Ok(response) if response.error.is_some() => {
                    images.push(ImageResult::error(name, response.error.unwrap_or_default()));
                }
                Ok(response) if response.is_match => {
                    let _ = fs::remove_file(&mask_path);
                    images.push(ImageResult::from_response(
                        name,
                        ImageStatus::Unchanged,
                        &response,
                        None,
                    ));
                }
                Ok(response) => {
                    let (status, mask) = match response.reason.as_deref() {
                        Some("layout-diff") => {
                            let _ = fs::remove_file(&mask_path);
                            (ImageStatus::LayoutChanged, None)
                        }
                        _ => (ImageStatus::Changed, Some(path_as_url(&mask_path))),
                    };
                    images.push(ImageResult::from_response(name, status, &response, mask));
                }
                Err(err) => {
                    images.push(ImageResult::error(name, format!("{err:#}")));
                    match OdiffServer::start(&binary_path) {
                        Ok(new_server) => server = new_server,
                        Err(_) => {
                            for remaining in &matched_paths[i + 1..] {
                                images.push(ImageResult::error(
                                    path_as_url(remaining),
                                    "Skipped: odiff server could not be restarted".to_owned(),
                                ));
                            }
                            break;
                        }
                    }
                }
            }
        }
    }

    for rel_path in &categorized.added {
        images.push(ImageResult::simple(
            path_as_url(rel_path),
            ImageStatus::Added,
        ));
    }

    for rel_path in &categorized.removed {
        images.push(ImageResult::simple(
            path_as_url(rel_path),
            ImageStatus::Removed,
        ));
    }

    for rel_path in &categorized.skipped {
        images.push(ImageResult::simple(
            path_as_url(rel_path),
            ImageStatus::Skipped,
        ));
    }

    let count = |statuses: &[ImageStatus]| {
        images
            .iter()
            .filter(|img| {
                statuses
                    .iter()
                    .any(|s| std::mem::discriminant(&img.status) == std::mem::discriminant(s))
            })
            .count()
    };
    let changed_count = count(&[ImageStatus::Changed, ImageStatus::LayoutChanged]);
    let unchanged_count = count(&[ImageStatus::Unchanged]);
    let added_count = count(&[ImageStatus::Added]);
    let removed_count = count(&[ImageStatus::Removed]);
    let skipped_count = count(&[ImageStatus::Skipped]);
    let errored_count = count(&[ImageStatus::Error]);
    let total = images.len();

    let report = DiffReport {
        base_dir: path_as_url(&base_dir),
        head_dir: path_as_url(&head_dir),
        output_dir: path_as_url(&output_dir),
        threshold,
        summary: DiffSummary {
            total,
            changed: changed_count,
            unchanged: unchanged_count,
            added: added_count,
            removed: removed_count,
            skipped: skipped_count,
            errored: errored_count,
        },
        images,
    };

    serde_json::to_writer_pretty(std::io::stdout(), &report)?;
    println!();

    eprintln!(
        "\nSummary: {total} total, {changed_count} changed, {unchanged_count} unchanged, {added_count} added, {removed_count} removed, {skipped_count} skipped, {errored_count} errored",
    );

    if fail_on_diff
        && (changed_count > 0 || errored_count > 0 || added_count > 0 || removed_count > 0)
    {
        let mut parts = Vec::new();
        if changed_count > 0 {
            parts.push(format!(
                "{changed_count} image{} differed from baseline",
                if changed_count == 1 { "" } else { "s" }
            ));
        }
        if added_count > 0 {
            parts.push(format!(
                "{added_count} image{} added",
                if added_count == 1 { " was" } else { "s were" }
            ));
        }
        if removed_count > 0 {
            parts.push(format!(
                "{removed_count} image{} removed",
                if removed_count == 1 { " was" } else { "s were" }
            ));
        }
        if errored_count > 0 {
            parts.push(format!(
                "{errored_count} image{} errored during comparison",
                if errored_count == 1 { "" } else { "s" }
            ));
        }
        bail!("{}", parts.join(", "));
    }

    Ok(())
}

fn collect_image_files(dir: &Path) -> Result<BTreeSet<PathBuf>> {
    let mut files = BTreeSet::new();
    for entry in WalkDir::new(dir).follow_links(true) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let is_image = path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
            IMAGE_EXTENSIONS
                .iter()
                .any(|ext| e.eq_ignore_ascii_case(ext))
        });
        if is_image {
            let rel = path.strip_prefix(dir)?.to_path_buf();
            files.insert(rel);
        }
    }
    Ok(files)
}

fn categorize_images(
    base: &BTreeSet<PathBuf>,
    head: &BTreeSet<PathBuf>,
    selective: bool,
) -> CategorizedImages {
    let base_only: BTreeSet<PathBuf> = base.difference(head).cloned().collect();
    if selective {
        CategorizedImages {
            matched: base.intersection(head).cloned().collect(),
            added: head.difference(base).cloned().collect(),
            removed: BTreeSet::new(),
            skipped: base_only,
        }
    } else {
        CategorizedImages {
            matched: base.intersection(head).cloned().collect(),
            added: head.difference(base).cloned().collect(),
            removed: base_only,
            skipped: BTreeSet::new(),
        }
    }
}

fn files_are_identical(a: &Path, b: &Path) -> Result<bool> {
    let meta_a = fs::metadata(a)?;
    let meta_b = fs::metadata(b)?;
    if meta_a.len() != meta_b.len() {
        return Ok(false);
    }
    let bytes_a = fs::read(a)?;
    let bytes_b = fs::read(b)?;
    Ok(bytes_a == bytes_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_all_matched() {
        let base: BTreeSet<PathBuf> = ["a.png", "b.png"].iter().map(PathBuf::from).collect();
        let head: BTreeSet<PathBuf> = ["a.png", "b.png"].iter().map(PathBuf::from).collect();
        let result = categorize_images(&base, &head, false);
        assert_eq!(result.matched.len(), 2);
        assert!(result.added.is_empty());
        assert!(result.removed.is_empty());
    }

    #[test]
    fn test_categorize_added_and_removed() {
        let base: BTreeSet<PathBuf> = ["a.png", "b.png"].iter().map(PathBuf::from).collect();
        let head: BTreeSet<PathBuf> = ["b.png", "c.png"].iter().map(PathBuf::from).collect();
        let result = categorize_images(&base, &head, false);
        assert_eq!(result.matched.len(), 1);
        assert!(result.matched.contains(&PathBuf::from("b.png")));
        assert_eq!(result.added.len(), 1);
        assert!(result.added.contains(&PathBuf::from("c.png")));
        assert_eq!(result.removed.len(), 1);
        assert!(result.removed.contains(&PathBuf::from("a.png")));
    }

    #[test]
    fn test_categorize_with_subdirs() {
        let base: BTreeSet<PathBuf> = ["sub/a.png", "sub/b.png", "root.png"]
            .iter()
            .map(PathBuf::from)
            .collect();
        let head: BTreeSet<PathBuf> = ["sub/a.png", "sub/c.png", "root.png"]
            .iter()
            .map(PathBuf::from)
            .collect();
        let result = categorize_images(&base, &head, false);
        assert_eq!(result.matched.len(), 2);
        assert!(result.matched.contains(&PathBuf::from("sub/a.png")));
        assert!(result.matched.contains(&PathBuf::from("root.png")));
        assert_eq!(result.added.len(), 1);
        assert!(result.added.contains(&PathBuf::from("sub/c.png")));
        assert_eq!(result.removed.len(), 1);
        assert!(result.removed.contains(&PathBuf::from("sub/b.png")));
    }

    #[test]
    fn test_categorize_empty_dirs() {
        let base: BTreeSet<PathBuf> = BTreeSet::new();
        let head: BTreeSet<PathBuf> = BTreeSet::new();
        let result = categorize_images(&base, &head, false);
        assert!(result.matched.is_empty());
        assert!(result.added.is_empty());
        assert!(result.removed.is_empty());
    }

    #[test]
    fn test_categorize_selective_skips_removed() {
        let base: BTreeSet<PathBuf> = ["a.png", "b.png", "c.png"]
            .iter()
            .map(PathBuf::from)
            .collect();
        let head: BTreeSet<PathBuf> = ["b.png"].iter().map(PathBuf::from).collect();
        let result = categorize_images(&base, &head, true);
        assert_eq!(result.matched.len(), 1);
        assert!(result.matched.contains(&PathBuf::from("b.png")));
        assert!(result.removed.is_empty());
        assert_eq!(result.skipped.len(), 2);
        assert!(result.skipped.contains(&PathBuf::from("a.png")));
        assert!(result.skipped.contains(&PathBuf::from("c.png")));
        assert!(result.added.is_empty());
    }

    #[test]
    fn test_categorize_non_selective_removes() {
        let base: BTreeSet<PathBuf> = ["a.png", "b.png", "c.png"]
            .iter()
            .map(PathBuf::from)
            .collect();
        let head: BTreeSet<PathBuf> = ["b.png"].iter().map(PathBuf::from).collect();
        let result = categorize_images(&base, &head, false);
        assert_eq!(result.matched.len(), 1);
        assert_eq!(result.removed.len(), 2);
        assert!(result.skipped.is_empty());
    }
}
