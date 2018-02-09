use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::File;
use std::iter::Fuse;
use std::path::{Path, PathBuf};

use clap::ArgMatches;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use sha1::Digest;
use symbolic_common::{ByteView, ObjectClass, ObjectKind};
use symbolic_debuginfo::FatObject;
use uuid::Uuid;
use walkdir::{DirEntry, IntoIter as WalkDirIter, WalkDir};
use zip::ZipWriter;
use zip::write::FileOptions;

use api::{Api, DSymFile};
use config::Config;
use prelude::*;
use utils::{copy_with_progress, invert_result, make_byte_progress_bar, TempFile, get_sha1_checksum};

/// Reference to an object file in the filesystem.
///
/// For Mach-O, this will always point to the fat object file containing this
/// object. Therefore, an identical `ObjectRef` can be returned multiple times
/// for each match within a fat object. Still, each file will only be uploaded
/// once.
#[derive(Debug)]
pub struct ObjectRef {
    /// The absolute path to the object file.
    pub path: PathBuf,
    /// SHA1 Checksum of the object file.
    pub checksum: Digest,
    /// File name of the object file without path.
    pub name: String,
    /// Size of the object file in bytes.
    pub size: u64,
}

/// A batch of found objects during walking.
pub type ObjectBatch = Vec<ObjectRef>;

/// Recursively searches a path for matching object files in batches.
///
/// The `BatchedObjectWalker` is initialized in a file system path and
/// recursively searches all directories for object files. To control
/// which files will be included, use the `object_kind`, `object_class`
/// and `file_extension` methods. By default, the walker includes all
/// objects it can find.
///
/// To search for objects with specific UUIDs, use the `object_uuid` or
/// `object_uuids` methods. The walker will open each object and dismiss
/// all files with a different UUID. By default, the walker will not
/// check the UUID.
///
/// The UUID of each found object is inserted into the `found` HashSet.
/// Once in this set, all subsequent objects with the same UUID will be
/// skipped.
///
/// Files are batched together until the batch hits a certain maximum
/// size. If the `max_batch_size` is not set, only a single batch will
/// be returned.
pub struct BatchedObjectWalker<'a> {
    path: PathBuf,
    iter: Fuse<WalkDirIter>,
    found: &'a mut BTreeSet<Uuid>,
    uuids: BTreeSet<Uuid>,
    kinds: BTreeSet<ObjectKind>,
    classes: BTreeSet<ObjectClass>,
    extensions: BTreeSet<OsString>,
    max_size: Option<u64>,
}

impl<'a> BatchedObjectWalker<'a> {
    /// Initializes a `BatchedObjectWalker` in the given path.
    ///
    /// The walker starts scanning this directory recursively and puts
    /// the UUID of every matching object file in the `found` set.
    pub fn new(path: PathBuf, found: &'a mut BTreeSet<Uuid>) -> Self {
        let iter = WalkDir::new(&path).into_iter().fuse();

        BatchedObjectWalker {
            path: path,
            iter: iter,
            found: found,
            uuids: BTreeSet::new(),
            kinds: BTreeSet::new(),
            classes: BTreeSet::new(),
            extensions: BTreeSet::new(),
            max_size: None,
        }
    }

    /// Add a `Uuid` to search for.
    ///
    /// By default, all UUIDs will be included.
    pub fn object_uuid(&mut self, uuid: Uuid) -> &mut Self {
        self.uuids.insert(uuid);
        self
    }

    /// Add `Uuid`s to search for.
    ///
    /// By default, all UUIDs will be included. If `uuids` is empty, this will
    /// not be changed.
    pub fn object_uuids<I>(&mut self, uuids: I) -> &mut Self
    where
        I: IntoIterator<Item = Uuid>,
    {
        self.uuids.extend(uuids);
        self
    }

    /// Add an `ObjectKind` to search for.
    ///
    /// By default, all object kinds will be included.
    pub fn object_kind(&mut self, kind: ObjectKind) -> &mut Self {
        self.kinds.insert(kind);
        self
    }

    /// Add `ObjectKind`s to search for.
    ///
    /// By default, all object kinds will be included. If `kinds` is empty, this
    /// will not be changed.
    pub fn object_kinds<I>(&mut self, kinds: I) -> &mut Self
    where
        I: IntoIterator<Item = ObjectKind>,
    {
        self.kinds.extend(kinds);
        self
    }

    /// Add an `ObjectClass` to search for.
    ///
    /// By default, all object classes will be included.
    pub fn object_class(&mut self, class: ObjectClass) -> &mut Self {
        self.classes.insert(class);
        self
    }

    /// Add `ObjectClass`es to search for.
    ///
    /// By default, all object classes will be included. If `kinds` is empty, this
    /// will not be changed.
    pub fn object_classes<I>(&mut self, classes: I) -> &mut Self
    where
        I: IntoIterator<Item = ObjectClass>,
    {
        self.classes.extend(classes);
        self
    }

    /// Add a file extension to search for.
    ///
    /// By default, all file extensions will be included.
    pub fn file_extension<S>(&mut self, extension: S) -> &mut Self
    where
        S: Into<OsString>,
    {
        self.extensions.insert(extension.into());
        self
    }

    /// Add a file extension to search for.
    ///
    /// By default, all file extensions will be included.
    pub fn file_extensions<I, S>(&mut self, extensions: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        for extension in extensions {
            self.extensions.insert(extension.into());
        }
        self
    }

    /// Set the maximum batch size in bytes.
    ///
    /// By default, batches are not limited in size and only one batch will
    /// ever be returned by the iterator.
    pub fn max_batch_size(&mut self, max_size: u64) -> &mut Self {
        self.max_size = Some(max_size);
        self
    }

    /// Determines if all searched UUIDs have been found or search should
    /// continue.
    fn found_all(&self) -> bool {
        if self.uuids.is_empty() {
            false
        } else {
            self.found.is_superset(&self.uuids)
        }
    }

    /// Determines if the maximum batch size has been reached or search should
    /// continue.
    fn is_filled(&self, batch: &ObjectBatch) -> bool {
        match self.max_size {
            Some(s) => batch.iter().map(|sym| sym.size).sum::<u64>() >= s,
            None => false,
        }
    }

    /// Determines if this UUID matches the search criteria.
    fn valid_uuid(&self, uuid: Uuid) -> bool {
        !self.found.contains(&uuid) && (self.uuids.is_empty() || self.uuids.contains(&uuid))
    }

    /// Determines if this file extension matches the search criteria.
    fn valid_extension(&self, ext: Option<&OsStr>) -> bool {
        self.extensions.is_empty() || ext.map_or(false, |e| self.extensions.contains(e.into()))
    }

    /// Determines if this `ObjectKind` matches the search criteria.
    fn valid_kind(&self, kind: ObjectKind) -> bool {
        self.kinds.is_empty() || self.kinds.contains(&kind)
    }

    /// Determines if this `ObjectClass` matches the search criteria.
    fn valid_class(&self, class: ObjectClass) -> bool {
        self.classes.is_empty() || self.classes.contains(&class)
    }

    /// Checks a single file if it is a matching object file.
    fn process_file(&mut self, entry: DirEntry, pb: &ProgressBar) -> Result<Option<ObjectRef>> {
        let meta = entry.metadata()?;
        if !meta.is_file() {
            // The WalkDir iterator will automatically recurse into directories
            return Ok(None);
        }

        let path = entry.path();
        if !self.valid_extension(path.extension()) {
            return Ok(None);
        }

        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            pb.set_message(name);
        }

        let data = ByteView::from_path(path)?;
        if !FatObject::peek(&data).ok().map_or(false, |k| self.valid_kind(k)) {
            return Ok(None);
        }

        let fat = FatObject::parse(data)?;
        let object = fat.get_object(0)?.unwrap();
        if !self.valid_class(object.class()) {
            return Ok(None);
        }

        let uuid = match object.uuid() {
            Some(uuid) => uuid,
            None => return Ok(None),
        };

        if !self.valid_uuid(uuid) {
            return Ok(None);
        }

        self.found.insert(uuid);
        Ok(Some(ObjectRef {
            path: path.to_path_buf(),
            checksum: get_sha1_checksum(fat.as_bytes())?,
            name: Path::new("DebugSymbols")
                .join(path.strip_prefix(&self.path).unwrap())
                .to_string_lossy()
                .into_owned(),
            size: meta.len(),
        }))
    }

    /// Walks files until the next matching object file is found.
    fn next_object(&mut self, pb: &ProgressBar) -> Result<Option<ObjectRef>> {
        while let Some(entry) = self.iter.next() {
            if let Some(sym) = self.process_file(entry?, pb)? {
                return Ok(Some(sym));
            }
        }

        Ok(None)
    }

    /// Collects objects into a batch.
    fn next_batch(&mut self) -> Result<Option<ObjectBatch>> {
        let pb = ProgressBar::new_spinner();
        pb.enable_steady_tick(100);
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("/|\\- ")
                .template(
                    "{spinner} Looking for symbols... {msg:.dim}\
                     \n  symbol files found: {prefix:.yellow}",
                ),
        );

        let mut batch = vec![];
        while !self.is_filled(&batch) && !self.found_all() {
            match self.next_object(&pb)? {
                Some(sym) => {
                    batch.push(sym);
                    pb.set_prefix(&format!("{}", batch.len()));
                }
                None => break,
            }
        }

        pb.finish_and_clear();
        Ok(if batch.len() == 0 { None } else { Some(batch) })
    }
}

impl<'a> Iterator for BatchedObjectWalker<'a> {
    type Item = Result<ObjectBatch>;

    fn next(&mut self) -> Option<Self::Item> {
        invert_result(self.next_batch())
    }
}

/// Options for uploading debug symbols to Sentry.
pub struct UploadOptions {
    api: Api,
    org: String,
    project: String,
    max_size: u64,
}

impl UploadOptions {
    /// Creates an `UploadOptions` from command line arguments.
    pub fn from_cli(matches: &ArgMatches) -> Result<UploadOptions> {
        let config = Config::get_current();
        let (org, project) = config.get_org_and_project(matches)?;
        let max_size = config.get_max_dsym_upload_size()?;

        Ok(UploadOptions {
            api: Api::new(),
            org: org,
            project: project,
            max_size,
        })
    }

    /// Returns the `Api` instance used to upload debug symbols.
    pub fn api(&self) -> &Api {
        &self.api
    }

    /// Returns the Sentry organization slug.
    pub fn org(&self) -> &str {
        &self.org
    }

    /// Returns the Sentry project slug.
    pub fn project(&self) -> &str {
        &self.project
    }

    /// Returns the maximum size for symbol uploads to Sentry.
    pub fn max_size(&self) -> u64 {
        self.max_size
    }
}

impl fmt::Debug for UploadOptions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("UploadOptions")
            .field("org", &self.org)
            .field("project", &self.project)
            .field("max_size", &self.max_size)
            .finish()
    }
}

/// Returns an object batch that only includes objects missing on Sentry.
fn filter_missing_syms(batch: ObjectBatch, context: &UploadOptions) -> Result<ObjectBatch> {
    info!("Checking for missing debug symbols: {:#?}", &batch);

    let missing_checksums = {
        let checksums = batch.iter().map(|ref s| s.checksum);
        context.api().find_missing_dsym_checksums(&context.org(), &context.project(), checksums)?
    };

    let missing = batch
        .into_iter()
        .filter(|sym| missing_checksums.contains(&sym.checksum))
        .collect();

    info!("Missing debug symbols: {:#?}", &missing);
    Ok(missing)
}

/// Compresses the given batch into a ZIP archive.
fn compress_syms(batch: ObjectBatch) -> Result<TempFile> {
    let total_bytes = batch.iter().map(|sym| sym.size).sum();
    let pb = make_byte_progress_bar(total_bytes);
    let tf = TempFile::new()?;
    let mut zip = ZipWriter::new(tf.open());

    for ref sym in batch {
        zip.start_file(sym.name.clone(), FileOptions::default())?;
        copy_with_progress(&pb, &mut File::open(&sym.path)?, &mut zip)?;
    }

    pb.finish_and_clear();
    Ok(tf)
}

/// Uploads the given debug symbols to Sentry.
fn upload_syms(batch: ObjectBatch, context: &UploadOptions) -> Result<Vec<DSymFile>> {
    println!("{} Compressing {} missing debug symbol files", style(">").dim(), style(batch.len()).yellow());
    let archive = compress_syms(batch)?;

    println!("{} Uploading debug symbol files", style(">").dim());
    Ok(context.api().upload_dsyms(&context.org(), &context.project(), archive.path())?)
}

/// Checks for missing symbols and uploads them to Sentry.
///
/// Returns the number of uploaded symbols.
pub fn process_batch(batch: ObjectBatch, context: &UploadOptions) -> Result<usize> {
    println!("{} Found {} symbol files.", style(">").dim(), style(batch.len()).yellow());

    let missing = filter_missing_syms(batch, context)?;
    if missing.len() == 0 {
        println!("{} Nothing to compress, all symbols are on the server", style(">").dim());
        println!("{} Nothing to upload", style(">").dim());
        return Ok(0);
    }

    let uploaded = upload_syms(missing, context)?;
    if uploaded.len() > 0 {
        println!("Newly uploaded debug symbols:");
        for sym in &uploaded {
            println!("  {} ({}; {})", style(&sym.uuid).dim(), &sym.object_name, sym.cpu_name);
        }
    }

    Ok(uploaded.len())
}
