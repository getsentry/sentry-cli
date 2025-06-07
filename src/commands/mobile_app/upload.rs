use std::io::Write;
use std::path::Path;
use anyhow::{anyhow, Context as _, Result};
use clap::{Arg, ArgAction, ArgMatches, Command};
use log::debug;
#[cfg(target_os = "macos")]
use std::ffi::CString;
#[cfg(target_os = "macos")]
use std::os::unix::ffi::OsStrExt;
#[cfg(target_os = "macos")]
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use walkdir::WalkDir;
use symbolic::common::ByteView;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::utils::args::ArgExt;
use crate::utils::fs::TempFile;
use crate::utils::mobile_app::{is_aab_file, is_apk_file, is_xcarchive_directory, is_zip_file};

pub fn make_command(command: Command) -> Command {
    command
        .about("[EXPERIMENTAL] Upload mobile app files to a project.")
        .org_arg()
        .project_arg(false)
        .arg(
            Arg::new("paths")
                .value_name("PATH")
                .help("The path to the mobile app files to upload. Supported files include Apk, Aab or XCArchive.")
                .num_args(1..)
                .action(ArgAction::Append),
        )
}

#[cfg(target_os = "macos")]
extern "C" {
    fn swift_inspect_asset_catalog(msg: *const std::os::raw::c_char);
}

#[cfg(target_os = "macos")]
pub fn inspect_asset_catalog<P: AsRef<Path>>(path: P) {
    // let rust_string = "/Users/noahmartin/Library/Developer/Xcode/DerivedData/HackerNews-dmsbmgkqxtdhuggaicvwnkihdwne/Build/Products/Release-iphonesimulator/HackerNews.app/Assets.car";
    let c_string = CString::new(path.as_ref().as_os_str().as_bytes()).expect("CString::new failed");
    unsafe {
        swift_inspect_asset_catalog(c_string.as_ptr());
    }
}

#[cfg(target_os = "macos")]
pub fn find_car_files(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)                   // discard I/O errors
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|ext| ext.eq("car"))
        })
        .map(|e| e.into_path())
        .collect()
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let path_strings = matches
        .get_many::<String>("paths")
        .expect("paths argument is required");

    debug!(
        "Starting mobile app upload for {} paths",
        path_strings.len()
    );

    let mut normalized_zips = vec![];
    for path_string in path_strings {
        let path: &Path = path_string.as_ref();
        debug!("Processing artifact at path: {}", path.display());

        if !path.exists() {
            return Err(anyhow!("Path does not exist: {}", path.display()));
        }

        let byteview = ByteView::open(path)?;
        debug!("Loaded file with {} bytes", byteview.len());

        #[cfg(target_os = "macos")]
        if is_apple_app(path) {
            // Find all asset catalogs
            let cars = find_car_files(path);
            for car in &cars {
                inspect_asset_catalog(car);
            }
        }

        validate_is_mobile_app(path, &byteview)?;

        let normalized_zip = if path.is_file() {
            debug!("Normalizing file: {}", path.display());
            normalize_file(path, &byteview).with_context(|| {
                format!(
                    "Failed to generate uploadable bundle for file {}",
                    path.display()
                )
            })?
        } else if path.is_dir() {
            debug!("Normalizing directory: {}", path.display());
            normalize_directory(path).with_context(|| {
                format!(
                    "Failed to generate uploadable bundle for directory {}",
                    path.display()
                )
            })?
        } else {
            Err(anyhow!(
                "Path {} is neither a file nor a directory, cannot upload",
                path.display()
            ))?
        };

        debug!(
            "Successfully normalized to: {}",
            normalized_zip.path().display()
        );
        normalized_zips.push(normalized_zip);
    }

    for zip in normalized_zips {
        println!("Created normalized zip at: {}", zip.path().display());
        // TODO: Upload the normalized zip to the chunked uploads API
    }
    eprintln!("Uploading mobile app files to a project is not yet implemented.");
    Ok(())
}

fn is_apple_app(path: &Path) -> bool {
    path.is_dir() && is_xcarchive_directory(path)
}

fn validate_is_mobile_app(path: &Path, bytes: &[u8]) -> Result<()> {
    debug!("Validating mobile app format for: {}", path.display());


    if is_apple_app(path) {
        debug!("Detected XCArchive directory");
        return Ok(());
    }

    // Check if the file is a zip file (then AAB or APK)
    if is_zip_file(bytes) {
        debug!("File is a zip, checking for AAB/APK format");
        if is_aab_file(bytes)? {
            debug!("Detected AAB file");
            return Ok(());
        }

        if is_apk_file(bytes)? {
            debug!("Detected APK file");
            return Ok(());
        }
    }

    debug!("File format validation failed");
    Err(anyhow!(
        "File is not a recognized mobile app format (APK, AAB, or XCArchive): {}",
        path.display()
    ))
}

// For APK and AAB files, we'll copy them directly into the zip
fn normalize_file(path: &Path, bytes: &[u8]) -> Result<TempFile> {
    debug!("Creating normalized zip for file: {}", path.display());

    let temp_file = TempFile::create()?;
    let mut zip = ZipWriter::new(temp_file.open()?);

    let file_name = path
        .file_name()
        .expect("Failed to get file name")
        .to_str()
        .with_context(|| format!("Failed to get relative path for {}", path.display()))?;

    debug!("Adding file to zip: {}", file_name);
    zip.start_file(file_name, SimpleFileOptions::default())?;
    zip.write_all(bytes)?;

    zip.finish()?;
    debug!("Successfully created normalized zip for file");
    Ok(temp_file)
}

// For XCArchive directories, we'll zip the entire directory
fn normalize_directory(path: &Path) -> Result<TempFile> {
    debug!("Creating normalized zip for directory: {}", path.display());

    let temp_file = TempFile::create()?;
    let mut zip = ZipWriter::new(temp_file.open()?);

    let mut file_count = 0;
    for entry in walkdir::WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_map(Result::ok)
    {
        let entry_path = entry.path();
        if entry_path.is_file() {
            let relative_path = entry_path.strip_prefix(path)?;
            debug!("Adding file to zip: {}", relative_path.display());

            zip.start_file(
                relative_path.to_string_lossy(),
                SimpleFileOptions::default(),
            )?;
            let file_byteview = ByteView::open(entry_path)?;
            zip.write_all(file_byteview.as_slice())?;
            file_count += 1;
        }
    }

    zip.finish()?;
    debug!(
        "Successfully created normalized zip for directory with {} files",
        file_count
    );
    Ok(temp_file)
}
