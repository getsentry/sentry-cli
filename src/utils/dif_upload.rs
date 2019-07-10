//! Searches, processes and uploads debug information files (DIFs). See
//! `DifUpload` for more information.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::iter::IntoIterator;
use std::mem::transmute;
use std::ops::Deref;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::slice::{Chunks, Iter};
use std::str;
use std::thread;

use console::style;
use failure::{bail, err_msg, Error, SyncFailure};
use indicatif::HumanBytes;
use log::{debug, info, warn};
use sha1::Digest;
use symbolic::common::{ByteView, DebugId, SelfCell, Uuid};
use symbolic::debuginfo::{sourcebundle::SourceBundleWriter, Archive, FileFormat, Object};
use walkdir::WalkDir;
use which::which;
use zip::{write::FileOptions, ZipArchive, ZipWriter};

use crate::api::{
    Api, ChunkUploadCapability, ChunkUploadOptions, ChunkedDifRequest, ChunkedFileState,
};
use crate::config::Config;
use crate::utils::chunks::{
    upload_chunks, BatchedSliceExt, Chunk, ItemSize, ASSEMBLE_POLL_INTERVAL,
};
use crate::utils::dif::DifFeatures;
use crate::utils::fs::{get_sha1_checksum, get_sha1_checksums, TempDir, TempFile};
use crate::utils::progress::{ProgressBar, ProgressStyle};
use crate::utils::ui::{copy_with_progress, make_byte_progress_bar};

/// A debug info file on the server.
pub use crate::api::DebugInfoFile;

/// Fallback maximum number of chunks in a batch for the legacy upload.
static MAX_CHUNKS: u64 = 64;

/// An iterator over chunks of data in a `ChunkedDifMatch` object.
///
/// This struct is returned by `ChunkedDifMatch::chunks`.
struct DifChunks<'a> {
    checksums: Iter<'a, Digest>,
    iter: Chunks<'a, u8>,
}

impl<'a> Iterator for DifChunks<'a> {
    type Item = Chunk<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.checksums.next(), self.iter.next()) {
            (Some(checksum), Some(data)) => Some(Chunk((*checksum, data))),
            (_, _) => None,
        }
    }
}

/// Contains backing data for a `DifMatch`.
///
/// This can be used to store the actual data that a `FatObject` might be
/// relying upon, such as temporary files or extracted archives. It will be
/// disposed along with a `DifMatch` once it is dropped.
#[derive(Debug)]
enum DifBacking {
    Temp(TempFile),
}

/// A handle to a debug information file found by `DifUpload`.
///
/// It contains a `FatObject` giving access to the metadata and contents of the
/// debug information file. `DifMatch::attachments` may contain supplemental
/// files used to further process this file, such as dSYM PLists.
struct DifMatch<'data> {
    _backing: Option<DifBacking>,
    object: SelfCell<ByteView<'data>, Object<'data>>,
    name: String,
    debug_id: Option<DebugId>,
    attachments: Option<BTreeMap<String, ByteView<'static>>>,
}

impl<'data> DifMatch<'data> {
    fn from_temp<S>(temp_file: TempFile, name: S) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        let buffer = ByteView::open(temp_file.path()).map_err(SyncFailure::new)?;

        Ok(DifMatch {
            _backing: Some(DifBacking::Temp(temp_file)),
            object: SelfCell::try_new(buffer, |b| Object::parse(unsafe { &*b }))?,
            name: name.into(),
            debug_id: None,
            attachments: None,
        })
    }

    /// Moves the specified temporary debug file to a safe location and assumes
    /// ownership. The file will be deleted in the file system when this
    /// `DifMatch` is dropped.
    ///
    /// The path must point to a `FatObject` containing exactly one `Object`.
    fn take_temp<P, S>(path: P, name: S) -> Result<Self, Error>
    where
        P: AsRef<Path>,
        S: Into<String>,
    {
        let temp_file = TempFile::take(path)?;
        Self::from_temp(temp_file, name)
    }

    /// Returns the parsed `Object` of this DIF.
    pub fn object(&self) -> &Object<'_> {
        self.object.get()
    }

    /// Returns the raw binary data of this DIF.
    pub fn data(&self) -> &[u8] {
        self.object().data()
    }

    /// Returns the size of of this DIF in bytes.
    pub fn size(&self) -> u64 {
        self.data().len() as u64
    }

    /// Returns the path of this DIF relative to the search origin.
    pub fn path(&self) -> &str {
        &self.name
    }

    /// Returns the name of this DIF, including its file extension.
    pub fn file_name(&self) -> &str {
        Path::new(self.path())
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("Generic")
    }

    /// Returns attachments of this DIF, if any.
    pub fn attachments(&self) -> Option<&BTreeMap<String, ByteView<'static>>> {
        self.attachments.as_ref()
    }

    /// Determines whether this file needs resolution of hidden symbols.
    pub fn needs_symbol_map(&self) -> bool {
        // XCode release archives and dSYM bundles downloaded from iTunes
        // Connect contain Swift library symbols. These have caused various
        // issues in the past, so we ignore them for now. In particular, there
        // are never any BCSymbolMaps generated for them and the DBGOriginalUUID
        // in the plist is the UUID of the original dsym file.
        //
        // We *might* have to locate the original library in the Xcode
        // distribution, then build a new non-fat dSYM file from it and patch
        // the the UUID.
        if self.file_name().starts_with("libswift") {
            return false;
        }

        match self.object() {
            Object::MachO(ref macho) => macho.requires_symbolmap(),
            _ => false,
        }
    }
}

impl<'data> fmt::Debug for DifMatch<'data> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DifMatch")
            .field("object", &self.object())
            .field("name", &self.name)
            .finish()
    }
}

/// A `DifMatch` with computed SHA1 checksum.
#[derive(Debug)]
struct HashedDifMatch<'data> {
    inner: DifMatch<'data>,
    checksum: Digest,
}

impl<'data> HashedDifMatch<'data> {
    /// Calculates the SHA1 checksum for the given DIF.
    fn from(inner: DifMatch<'data>) -> Result<Self, Error> {
        let checksum = get_sha1_checksum(inner.data())?;
        Ok(HashedDifMatch { inner, checksum })
    }

    /// Returns the SHA1 checksum of this DIF.
    fn checksum(&self) -> Digest {
        self.checksum
    }
}

impl<'data> Deref for HashedDifMatch<'data> {
    type Target = DifMatch<'data>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'data> ItemSize for HashedDifMatch<'data> {
    fn size(&self) -> u64 {
        self.deref().size()
    }
}

/// A chunked `DifMatch` with computed SHA1 checksums.
#[derive(Debug)]
struct ChunkedDifMatch<'data> {
    inner: HashedDifMatch<'data>,
    chunks: Vec<Digest>,
    chunk_size: u64,
}

impl<'data> ChunkedDifMatch<'data> {
    /// Slices the DIF into chunks of `chunk_size` bytes each, and computes SHA1
    /// checksums for every chunk as well as the entire DIF.
    pub fn from(inner: DifMatch<'data>, chunk_size: u64) -> Result<Self, Error> {
        let (checksum, chunks) = get_sha1_checksums(inner.data(), chunk_size)?;
        Ok(ChunkedDifMatch {
            inner: HashedDifMatch { inner, checksum },
            chunks,
            chunk_size,
        })
    }

    /// Returns an iterator over all chunk checksums.
    pub fn checksums(&self) -> Iter<'_, Digest> {
        self.chunks.iter()
    }

    /// Returns an iterator over all `DifChunk`s.
    pub fn chunks(&self) -> DifChunks<'_> {
        DifChunks {
            checksums: self.checksums(),
            iter: self.data().chunks(self.chunk_size as usize),
        }
    }

    /// Creates a tuple which can be collected into a `ChunkedDifRequest`.
    pub fn to_assemble(&self) -> (Digest, ChunkedDifRequest<'_>) {
        (
            self.checksum(),
            ChunkedDifRequest {
                name: self.file_name(),
                debug_id: self.debug_id,
                chunks: &self.chunks,
            },
        )
    }
}

impl<'data> Deref for ChunkedDifMatch<'data> {
    type Target = HashedDifMatch<'data>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'data> ItemSize for ChunkedDifMatch<'data> {
    fn size(&self) -> u64 {
        self.deref().size()
    }
}

type ZipFileArchive = ZipArchive<BufReader<File>>;

/// A handle to the source of a potential `DifMatch` used inside `search_difs`.
///
/// The primary use of this handle is to resolve files relative to the debug
/// information file and store them in `DifMatch::attachments`. These could be
/// companion files or metadata files needed to process the DIFs in sentry-cli,
/// or later even on Sentry.
#[derive(Debug)]
enum DifSource<'a> {
    /// A file located in the file system
    FileSystem(&'a Path),
    /// An entry in a ZIP file
    Zip(&'a mut ZipFileArchive, &'a str),
}

impl<'a> DifSource<'a> {
    /// Resolves a file relative to the directory of `base`, stripping of the
    /// file name.
    fn get_relative_fs(base: &Path, path: &Path) -> Option<ByteView<'static>> {
        // Use parent() to get to the directory and then move relative from
        // there. ByteView will internally cannonicalize the path and resolve
        // symlinks.
        base.parent()
            .and_then(|p| ByteView::open(p.join(path)).ok())
    }

    /// Extracts a file relative to the directory of `name`, stripping of the
    /// file name.
    fn get_relative_zip(
        zip: &mut ZipFileArchive,
        name: &str,
        path: &Path,
    ) -> Option<ByteView<'static>> {
        // There is no built-in utility that normalizes paths without access to
        // the file system. We start by removing the file name from the given
        // path and then start to manually resolve the path components to a
        // final path.
        let mut zip_path = PathBuf::from(name);
        zip_path.pop();

        for component in path.components() {
            match component {
                Component::ParentDir => {
                    zip_path.pop();
                }
                Component::Normal(p) => {
                    zip_path.push(p);
                }
                _ => {
                    // `Component::CurDir` leaves the path as-is, and the
                    // remaining `Component::RootDir` and `Component::Prefix` do
                    // not make sense in ZIP files.
                }
            }
        }

        zip_path
            .to_str()
            .and_then(|name| zip.by_name(name).ok())
            .and_then(|f| ByteView::read(f).ok())
    }

    /// Resolves a file relative to this source and reads it into a `ByteView`.
    ///
    /// The target is always resolved relative to the directory of the source,
    /// excluding its file name. The path "../changed" relative to a source
    /// pointing to "path/to/file" will resolve in "path/changed".
    ///
    /// The returned ByteView will allow random-access to the data until it is
    /// disposed. If the source points to a ZIP file, the target is fully read
    /// into a memory buffer. See `ByteView::from_reader` for more information.
    pub fn get_relative<P>(&mut self, path: P) -> Option<ByteView<'static>>
    where
        P: AsRef<Path>,
    {
        match *self {
            DifSource::FileSystem(base) => Self::get_relative_fs(base, path.as_ref()),
            DifSource::Zip(ref mut zip, name) => Self::get_relative_zip(*zip, name, path.as_ref()),
        }
    }
}

/// Information returned by `assemble_difs` containing flat lists of incomplete
/// DIFs and their missing chunks.
type MissingDifsInfo<'data, 'm> = (Vec<&'m ChunkedDifMatch<'data>>, Vec<Chunk<'m>>);

/// Verifies that the given path contains a ZIP file and opens it.
fn try_open_zip<P>(path: P) -> Result<Option<ZipFileArchive>, Error>
where
    P: AsRef<Path>,
{
    if path.as_ref().extension() != Some("zip".as_ref()) {
        return Ok(None);
    }

    let mut magic: [u8; 2] = [0; 2];
    let mut file = File::open(path)?;
    if file.read_exact(&mut magic).is_err() {
        // Catch empty or single-character files
        return Ok(None);
    }

    file.seek(SeekFrom::Start(0))?;
    Ok(match &magic {
        b"PK" => Some(ZipArchive::new(BufReader::new(file))?),
        _ => None,
    })
}

/// Searches the given ZIP for potential DIFs and passes them to the callback.
///
/// To avoid unnecessary file operations, the file extension is already checked
/// for every entry before opening it.
///
/// This function will not recurse into ZIPs contained in this ZIP.
fn walk_difs_zip<F>(mut zip: ZipFileArchive, options: &DifUpload, mut func: F) -> Result<(), Error>
where
    F: FnMut(DifSource<'_>, String, ByteView<'static>) -> Result<(), Error>,
{
    for index in 0..zip.len() {
        let (name, buffer) = {
            let zip_file = zip.by_index(index)?;
            let name = zip_file.name().to_string();

            if !options.valid_extension(Path::new(&name).extension()) {
                continue;
            }

            (name, ByteView::read(zip_file).map_err(SyncFailure::new)?)
        };

        func(DifSource::Zip(&mut zip, &name), name.clone(), buffer)?;
    }

    Ok(())
}

/// Recursively searches the given location for potential DIFs and passes them
/// to the callback.
///
/// If `DifUpload::allow_zips` is set, then this function will attempt to open
/// the ZIP and search it for DIFs as well, however not recursing further into
/// nested ZIPs.
///
/// To avoid unnecessary file operations, the file extension is already checked
/// for every entry before opening it.
fn walk_difs_directory<F, P>(location: P, options: &DifUpload, mut func: F) -> Result<(), Error>
where
    P: AsRef<Path>,
    F: FnMut(DifSource<'_>, String, ByteView<'static>) -> Result<(), Error>,
{
    let location = location.as_ref();
    let directory = if location.is_dir() {
        location
    } else {
        location.parent().unwrap()
    };

    debug!("searching location {}", location.display());
    for entry in WalkDir::new(location).into_iter().filter_map(Result::ok) {
        if !entry.metadata()?.is_file() {
            // Walkdir recurses automatically into folders
            continue;
        }

        let path = entry.path();
        match try_open_zip(path) {
            Ok(Some(zip)) => {
                debug!("searching zip archive {}", path.display());
                walk_difs_zip(zip, options, &mut func)?;
                debug!("finished zip archive {}", path.display());
                continue;
            }
            Err(e) => {
                debug!("skipping zip archive {}", path.display());
                debug!("error: {}", e);
                continue;
            }
            Ok(None) => {
                // this is not a zip archive
            }
        }

        if !options.valid_extension(path.extension()) {
            continue;
        }

        let buffer = ByteView::open(path).map_err(SyncFailure::new)?;
        let name = path
            .strip_prefix(directory)
            .unwrap()
            .to_string_lossy()
            .into_owned();

        func(DifSource::FileSystem(path), name, buffer)?;
    }

    debug!("finished location {}", directory.display());
    Ok(())
}

/// Searches for mapping PLists next to the given `source`. It returns a mapping
/// of Plist name to owning buffer of the file's contents. This function should
/// only be called for dSYMs.
fn find_uuid_plists(
    object: &Object<'_>,
    source: &mut DifSource<'_>,
) -> Option<BTreeMap<String, ByteView<'static>>> {
    let uuid = object.debug_id().uuid();
    if uuid.is_nil() {
        return None;
    }

    // When uploading an XCode build archive to iTunes Connect, Apple will
    // re-build the app for different architectures, causing new UUIDs in the
    // final bundle. To allow mapping back to the original symbols, it adds
    // PList files in the `Resources` folder (one level above the binary) that
    // contains the original UUID, one for each object contained in the fat
    // object.
    //
    // The folder structure looks like this:
    //
    //     App.dSYM
    //     ├─ Info.plist
    //     └─ Resources
    //        ├─ 1B205CD0-67D0-4D69-A0FA-C6BDDDB2A609.plist
    //        ├─ 1C228684-3EE5-472B-AB8D-29B3FBF63A70.plist
    //        └─ DWARF
    //           └─ App
    let plist_name = format!("{:X}.plist", uuid.to_hyphenated_ref());
    let plist = match source.get_relative(format!("../{}", &plist_name)) {
        Some(plist) => plist,
        None => return None,
    };

    let mut plists = BTreeMap::new();
    plists.insert(plist_name, plist);
    Some(plists)
}

/// Patch debug identifiers for PDBs where the corresponding PE specifies a different age.
fn fix_pdb_ages(difs: &mut [DifMatch<'_>], age_overrides: &BTreeMap<Uuid, u32>) {
    for dif in difs {
        if dif.object().file_format() != FileFormat::Pdb {
            continue;
        }

        let debug_id = dif.object().debug_id();
        let age = match age_overrides.get(&debug_id.uuid()) {
            Some(age) => *age,
            None => continue,
        };

        if age == debug_id.appendix() {
            continue;
        }

        log::debug!(
            "overriding age for {} ({} -> {})",
            dif.name,
            debug_id.appendix(),
            age
        );

        dif.debug_id = Some(DebugId::from_parts(debug_id.uuid(), age));
    }
}

/// Searches matching debug information files.
fn search_difs(options: &DifUpload) -> Result<Vec<DifMatch<'static>>, Error> {
    let progress_style = ProgressStyle::default_spinner().template(
        "{spinner} Searching for debug symbol files...\
         \n  found {prefix:.yellow} {msg:.dim}",
    );

    let progress = ProgressBar::new_spinner();
    progress.enable_steady_tick(100);
    progress.set_style(progress_style);

    let mut age_overrides = BTreeMap::new();
    let mut collected = Vec::new();
    for base_path in &options.paths {
        walk_difs_directory(base_path, options, |mut source, name, buffer| {
            progress.set_message(&name);

            // Try to parse a potential object file. If this is not possible,
            // then we're not dealing with an object file, thus silently
            // skipping it.
            let format = Archive::peek(&buffer);

            // Override this behavior for PE files. Their debug identifier is
            // needed in case PDBs should be uploaded to fix an eventual age
            // mismatch
            let should_override_age =
                format == FileFormat::Pe && options.valid_format(FileFormat::Pdb);

            if !should_override_age && !options.valid_format(format) {
                return Ok(());
            }

            debug!("trying to parse dif {}", name);
            let archive = match Archive::parse(&buffer) {
                Ok(archive) => archive,
                Err(e) => {
                    warn!("Skipping invalid debug file {}: {}", name, e);
                    return Ok(());
                }
            };

            // Each `FatObject` might contain multiple matching objects, each of
            // which needs to retain a reference to the original fat file. We
            // create a shared instance here and clone it into `DifMatche`s
            // below.
            for object in archive.objects() {
                // Silently skip all objects that we cannot process. This can
                // happen due to invalid object files, which we then just
                // discard rather than stopping the scan.
                let object = match object {
                    Ok(object) => object,
                    Err(_) => continue,
                };

                // Objects without debug id will be skipped altogether. While frames
                // during symbolication might be lacking debug identifiers,
                // Sentry requires object files to have one during upload.
                let id = object.debug_id();
                if id.is_nil() {
                    continue;
                }

                // Store a mapping of "age" values for all encountered PE files,
                // regardless of whether they will be uploaded. This is used later
                // to fix up PDB files.
                if should_override_age {
                    age_overrides.insert(id.uuid(), id.appendix());

                    // Skip if this object was only retained for the PDB override.
                    if !options.valid_format(format) {
                        continue;
                    }
                }

                // We can only process objects with features, such as a symbol
                // table or debug information. If this object has no features,
                // Sentry cannot process it and so we skip the upload. If object
                // features were specified, this will skip all other objects.
                if !options.valid_features(&object) {
                    continue;
                }

                // Skip this object if we're only looking for certain IDs.
                if !options.valid_id(id) {
                    continue;
                }

                // Skip this entire file if it exceeds the maximum allowed file size.
                let file_size = object.data().len() as u64;
                if file_size > options.max_file_size {
                    warn!(
                        "Skipping debug file since it exceeds {}: {} ({})",
                        HumanBytes(options.max_file_size),
                        name,
                        HumanBytes(file_size),
                    );
                    break;
                }

                // Invoke logic to retrieve attachments specific to the kind
                // of object file. These are used for processing. Since only
                // dSYMs equire processing currently, all other kinds are
                // skipped.
                let attachments = match object.file_format() {
                    FileFormat::MachO => find_uuid_plists(&object, &mut source),
                    _ => None,
                };

                // We retain the buffer and the borrowed object in a new SelfCell. This is
                // incredibly unsafe, but in our case it is fine, since the SelfCell owns the same
                // buffer that was used to retrieve the object.
                let cell = unsafe { SelfCell::from_raw(buffer.clone(), transmute(object)) };

                collected.push(DifMatch {
                    _backing: None,
                    object: cell,
                    name: name.clone(),
                    debug_id: None,
                    attachments,
                });

                progress.set_prefix(&collected.len().to_string());
            }

            Ok(())
        })?;
    }

    if !age_overrides.is_empty() {
        fix_pdb_ages(&mut collected, &age_overrides);
    }

    progress.finish_and_clear();
    println!(
        "{} Found {} debug information {}",
        style(">").dim(),
        style(collected.len()).yellow(),
        match collected.len() {
            1 => "file",
            _ => "files",
        }
    );

    Ok(collected)
}

/// Resolves BCSymbolMaps and replaces hidden symbols in a `DifMatch` using
/// `dsymutil`. If successful, this will return a new `DifMatch` based on a
/// temporary file. The original dSYM is not touched.
///
/// Note that this process copies the file to a temporary location and might
/// incur significant I/O for larger debug files.
fn resolve_hidden_symbols<'a>(dif: DifMatch<'a>, symbol_map: &Path) -> Result<DifMatch<'a>, Error> {
    if dif.attachments.is_none() {
        println!(
            "{} {}: Could not locate UUID mapping for {}",
            style(">").dim(),
            style("Warning").red(),
            style(dif.file_name()).yellow(),
        );
        return Ok(dif);
    }

    // We need to rebuild the Resources folder of a dSYM structure in a temp
    // directory that is guaranteed to be deleted after this operation. The
    // Info.plist is not needed for this operation:
    //     Resources
    //     ├─ 1B205CD0-67D0-4D69-A0FA-C6BDDDB2A609.plist
    //     ├─ 1C228684-3EE5-472B-AB8D-29B3FBF63A70.plist
    //     └─ DWARF
    //        └─ ObjectFile
    let temp_dir = TempDir::create()?;
    fs::create_dir_all(temp_dir.path().join("DWARF"))?;

    // Copy the object file binary
    let temp_path = temp_dir.path().join("DWARF").join(dif.file_name());
    let mut temp_file = File::create(&temp_path)?;
    temp_file.write_all(dif.data())?;
    temp_file.sync_data()?;

    // Copy the UUID plists
    for (name, view) in dif.attachments().unwrap() {
        let mut plist = File::create(temp_dir.path().join(name))?;
        plist.write_all(&view)?;
        plist.sync_data()?;
    }

    let output = Command::new("dsymutil")
        .arg("-symbol-map")
        .arg(symbol_map)
        .arg(&temp_path)
        .output()?;

    if !output.status.success() {
        if let Ok(error) = str::from_utf8(&output.stderr) {
            bail!("Could not resolve BCSymbolMaps: {}", error);
        } else {
            bail!("Could not resolve BCSymbolMaps due to an unknown error");
        }
    }

    // Take ownership of the modified (fat) object file and move it somewhere
    // else so it is safe to delete the temp directory.
    DifMatch::take_temp(temp_path, dif.path())
}

/// Runs all `DifMatch` objects through the provided callback and displays a
/// progress bar while doing so.
///
/// ```
/// prepare_difs(processed, |m| HashedDifMatch::from(m))?
/// ```
fn prepare_difs<'data, F, T>(items: Vec<DifMatch<'data>>, mut func: F) -> Result<Vec<T>, Error>
where
    F: FnMut(DifMatch<'data>) -> Result<T, Error>,
{
    let progress_style = ProgressStyle::default_bar().template(
        "{prefix:.dim} Preparing for upload... {msg:.dim}\
         \n{wide_bar}  {pos}/{len}",
    );

    let progress = ProgressBar::new(items.len() as u64);
    progress.set_style(progress_style);
    progress.set_prefix(">");

    let mut calculated = Vec::new();
    for item in items {
        progress.inc(1);
        progress.set_message(item.path());
        calculated.push(func(item)?);
    }

    progress.finish_and_clear();
    println!(
        "{} Prepared debug information {} for upload",
        style(">").dim(),
        match calculated.len() {
            1 => "file",
            _ => "files",
        }
    );

    Ok(calculated)
}

/// Resolves BCSymbolMaps for all debug files with hidden symbols. All other
/// files are not touched. Note that this only applies to Apple dSYMs.
///
/// If there are debug files with hidden symbols but no `symbol_map` path is
/// given, a warning is emitted.
fn process_symbol_maps<'a>(
    difs: Vec<DifMatch<'a>>,
    symbol_map: Option<&Path>,
) -> Result<Vec<DifMatch<'a>>, Error> {
    let (with_hidden, mut without_hidden): (Vec<_>, _) =
        difs.into_iter().partition(DifMatch::needs_symbol_map);

    if with_hidden.is_empty() {
        return Ok(without_hidden);
    }

    let symbol_map = match symbol_map {
        Some(path) => path,
        _ => {
            println!(
                "{} {}: Found {} symbol files with hidden symbols (need BCSymbolMaps)",
                style(">").dim(),
                style("Warning").red(),
                style(with_hidden.len()).yellow()
            );

            without_hidden.extend(with_hidden);
            return Ok(without_hidden);
        }
    };

    let len = with_hidden.len();
    let progress_style = ProgressStyle::default_bar().template(
        "{prefix:.dim} Resolving BCSymbolMaps... {msg:.dim}\
         \n{wide_bar}  {pos}/{len}",
    );

    let progress = ProgressBar::new(len as u64);
    progress.set_style(progress_style);
    progress.set_prefix(">");

    for dif in with_hidden {
        progress.inc(1);
        progress.set_message(dif.path());
        without_hidden.push(resolve_hidden_symbols(dif, symbol_map)?);
    }

    progress.finish_and_clear();
    println!(
        "{} Resolved BCSymbolMaps for {} debug information {}",
        style(">").dim(),
        style(len).yellow(),
        match len {
            1 => "file",
            _ => "files",
        }
    );

    Ok(without_hidden)
}

/// Resolves BCSymbolMaps for all debug files with hidden symbols. All other
/// files are not touched. Note that this only applies to Apple dSYMs.
///
/// If there are debug files with hidden symbols but no `symbol_map` path is
/// given, a warning is emitted.
fn create_source_bundles<'a>(difs: &[DifMatch<'a>]) -> Result<Vec<DifMatch<'a>>, Error> {
    let mut source_bundles = Vec::new();

    let progress_style = ProgressStyle::default_bar().template(
        "{prefix:.dim} Resolving source code... {msg:.dim}\
         \n{wide_bar}  {pos}/{len}",
    );

    let progress = ProgressBar::new(difs.len() as u64);
    progress.set_style(progress_style);
    progress.set_prefix(">");

    for dif in difs {
        progress.inc(1);
        progress.set_message(dif.path());

        let object = dif.object();
        if object.has_sources() {
            // Do not create standalone source bundles if the original object already contains
            // source code. This would just store duplicate information in Sentry.
            continue;
        }

        let temp_file = TempFile::create()?;
        let writer = SourceBundleWriter::start(BufWriter::new(temp_file.open()?))?;

        // Resolve source files from the object and write their contents into the archive. Skip to
        // upload this bundle if no source could be written. This can happen if there is no file or
        // line information in the object file, or if none of the files could be resolved.
        let written = writer.write_object(object, dif.file_name())?;
        if !written {
            continue;
        }

        let mut source_bundle = DifMatch::from_temp(temp_file, dif.path())?;
        source_bundle.debug_id = dif.debug_id;
        source_bundles.push(source_bundle);
    }

    let len = source_bundles.len();
    progress.finish_and_clear();
    println!(
        "{} Resolved source code for {} debug information {}",
        style(">").dim(),
        style(len).yellow(),
        match len {
            1 => "file",
            _ => "files",
        }
    );

    Ok(source_bundles)
}

/// Calls the assemble endpoint and returns the state for every `DifMatch` along
/// with info on missing chunks.
///
/// The returned value containes separate vectors for incomplete DIFs and
/// missing chunks for convenience.
fn try_assemble_difs<'data, 'm>(
    difs: &'m [ChunkedDifMatch<'data>],
    options: &DifUpload,
) -> Result<MissingDifsInfo<'data, 'm>, Error> {
    let api = Api::current();
    let request = difs.iter().map(ChunkedDifMatch::to_assemble).collect();
    let response = api.assemble_difs(&options.org, &options.project, &request)?;

    // We map all DIFs by their checksum, so we can access them faster when
    // iterating through the server response below. Since the caller will invoke
    // this function multiple times (most likely twice), this operation is
    // performed twice with the same data. While this is redundant, it is also
    // fast enough and keeping it here makes the `try_assemble_difs` interface
    // nicer.
    let difs_by_checksum = difs
        .iter()
        .map(|m| (m.checksum, m))
        .collect::<BTreeMap<_, _>>();

    let mut difs = Vec::new();
    let mut chunks = Vec::new();
    for (checksum, ref file_response) in response {
        let chunked_match = *difs_by_checksum
            .get(&checksum)
            .ok_or_else(|| err_msg("Server returned unexpected checksum"))?;

        match file_response.state {
            ChunkedFileState::Error => {
                // One of the files could not be uploaded properly and resulted
                // in an error. We include this file in the return value so that
                // it shows up in the final report.
                difs.push(chunked_match);
            }
            ChunkedFileState::Assembling => {
                // This file is currently assembling. The caller will have to poll this file later
                // until it either resolves or errors.
                difs.push(chunked_match);
            }
            ChunkedFileState::NotFound => {
                // Assembling for one of the files has not started because some
                // (or all) of its chunks have not been found. We report its
                // missing chunks to the caller and then continue. The caller
                // will have to call `try_assemble_difs` again after uploading
                // them.
                let mut missing_chunks = chunked_match
                    .chunks()
                    .filter(|&Chunk((c, _))| file_response.missing_chunks.contains(&c))
                    .peekable();

                // Usually every file that is NotFound should also contain a set
                // of missing chunks. However, if we tried to upload an empty
                // file or the server returns an invalid response, we need to
                // make sure that this match is not included in the missing
                // difs.
                if missing_chunks.peek().is_some() {
                    difs.push(chunked_match);
                }

                chunks.extend(missing_chunks);
            }
            _ => {
                // This file has already finished. No action required anymore.
            }
        }
    }

    Ok((difs, chunks))
}

/// Concurrently uploads chunks specified in `missing_info` in batches. The
/// batch size and number of concurrent requests is controlled by
/// `chunk_options`.
///
/// This function blocks until all chunks have been uploaded.
fn upload_missing_chunks(
    missing_info: &MissingDifsInfo<'_, '_>,
    chunk_options: &ChunkUploadOptions,
) -> Result<(), Error> {
    let &(ref difs, ref chunks) = missing_info;

    // Chunks might be empty if errors occurred in a previous upload. We do
    // not need to render a progress bar or perform an upload in this case.
    if chunks.is_empty() {
        return Ok(());
    }

    let progress_style = ProgressStyle::default_bar().template(&format!(
        "{} Uploading {} missing debug information file{}...\
         \n{{wide_bar}}  {{bytes}}/{{total_bytes}} ({{eta}})",
        style(">").dim(),
        style(difs.len().to_string()).yellow(),
        if difs.len() == 1 { "" } else { "s" }
    ));

    upload_chunks(chunks, chunk_options, progress_style)?;

    println!(
        "{} Uploaded {} missing debug information {}",
        style(">").dim(),
        style(difs.len().to_string()).yellow(),
        match difs.len() {
            1 => "file",
            _ => "files",
        }
    );

    Ok(())
}

/// Renders the given detail string to the command line. If the `detail` is
/// either missing or empty, the optional fallback will be used.
fn render_detail(detail: &Option<String>, fallback: Option<&str>) {
    let mut string = match *detail {
        Some(ref string) => string.as_str(),
        None => "",
    };

    if string.is_empty() && fallback.is_some() {
        string = fallback.unwrap();
    }

    for line in string.lines() {
        if !line.is_empty() {
            println!("        {}", style(line).dim());
        }
    }
}

/// Polls the assemble endpoint until all DIFs have either completed or errored. Returns a list of
/// `DebugInfoFile`s that have been created successfully and also prints a summary to the user.
///
/// This function assumes that all chunks have been uploaded successfully. If there are still
/// missing chunks in the assemble response, this likely indicates a bug in the server.
fn poll_dif_assemble(
    difs: &[&ChunkedDifMatch<'_>],
    options: &DifUpload,
) -> Result<(Vec<DebugInfoFile>, bool), Error> {
    let progress_style = ProgressStyle::default_bar().template(
        "{prefix:.dim} Processing files...\
         \n{wide_bar}  {pos}/{len}",
    );

    let api = Api::current();
    let progress = ProgressBar::new(difs.len() as u64);
    progress.set_style(progress_style);
    progress.set_prefix(">");

    let request = difs.iter().map(|d| d.to_assemble()).collect();
    let response = loop {
        let response = api.assemble_difs(&options.org, &options.project, &request)?;

        let chunks_missing = response
            .values()
            .any(|r| r.state == ChunkedFileState::NotFound);
        if chunks_missing {
            return Err(err_msg(
                "Some uploaded files are now missing on the server. Please retry by running \
                 `sentry-cli upload-dif` again. If this problem persists, please report a bug.",
            ));
        }

        let pending = response.iter().filter(|&(_, r)| r.state.pending()).count();
        progress.set_position((difs.len() - pending) as u64);

        if pending == 0 {
            break response;
        }

        thread::sleep(ASSEMBLE_POLL_INTERVAL);
    };

    progress.finish_and_clear();
    println!("{} File processing complete:\n", style(">").dim());
    let (mut successes, errors): (Vec<_>, _) =
        response.into_iter().partition(|&(_, ref r)| r.state.ok());

    // Print a summary of all successes first, so that errors show up at the
    // bottom for the user
    successes.sort_by(|a, b| {
        let name_a =
            a.1.dif
                .as_ref()
                .map(|x| x.object_name.as_str())
                .unwrap_or("");
        let name_b =
            b.1.dif
                .as_ref()
                .map(|x| x.object_name.as_str())
                .unwrap_or("");
        name_a.cmp(name_b)
    });

    for &(_, ref success) in &successes {
        // Silently skip all OK entries without a "dif" record since the server
        // will always return one.
        if let Some(ref dif) = success.dif {
            println!(
                "     {} {} ({}; {}{})",
                style("OK").green(),
                style(&dif.id()).dim(),
                dif.object_name,
                dif.cpu_name,
                dif.data
                    .kind
                    .map(|c| format!(" {:#}", c))
                    .unwrap_or_default()
            );

            render_detail(&success.detail, None);
        }
    }

    // Print a summary of all errors at the bottom.
    let difs_by_checksum: BTreeMap<_, _> = difs.iter().map(|m| (m.checksum, m)).collect();
    let mut errored = vec![];
    for (checksum, error) in errors {
        let dif = difs_by_checksum
            .get(&checksum)
            .ok_or_else(|| err_msg("Server returned unexpected checksum"))?;
        errored.push((dif, error));
    }
    errored.sort_by_key(|x| x.0.file_name());

    let has_errors = !errored.is_empty();
    for (dif, error) in errored {
        println!("  {} {}", style("ERROR").red(), dif.file_name());
        render_detail(&error.detail, Some("An unknown error occurred"));
    }

    // Return only successful uploads
    Ok((
        successes.into_iter().filter_map(|(_, r)| r.dif).collect(),
        has_errors,
    ))
}

/// Uploads debug info files using the chunk-upload endpoint.
fn upload_difs_chunked(
    options: &DifUpload,
    chunk_options: &ChunkUploadOptions,
) -> Result<(Vec<DebugInfoFile>, bool), Error> {
    // Search for debug files in the file system and ZIPs
    let found = search_difs(options)?;
    if found.is_empty() {
        println!(
            "{} No debug debug information files found",
            style(">").dim()
        );
        return Ok(Default::default());
    }

    // Try to resolve BCSymbolMaps
    let symbol_map = options.symbol_map.as_ref().map(PathBuf::as_path);
    let mut processed = process_symbol_maps(found, symbol_map)?;

    // Resolve source code context if specified
    if options.include_sources {
        let source_bundles = create_source_bundles(&processed)?;
        processed.extend(source_bundles);
    }

    // Calculate checksums and chunks
    let chunked = prepare_difs(processed, |m| {
        ChunkedDifMatch::from(m, chunk_options.chunk_size)
    })?;

    // Upload missing chunks to the server and remember incomplete difs
    let missing_info = try_assemble_difs(&chunked, options)?;
    upload_missing_chunks(&missing_info, chunk_options)?;

    // Only if DIFs were missing, poll until assembling is complete
    let (missing_difs, _) = missing_info;
    if !missing_difs.is_empty() {
        poll_dif_assemble(&missing_difs, options)
    } else {
        println!(
            "{} Nothing to upload, all files are on the server",
            style(">").dim()
        );

        Ok((Default::default(), false))
    }
}

/// Returns debug files missing on the server.
fn get_missing_difs<'data>(
    objects: Vec<HashedDifMatch<'data>>,
    options: &DifUpload,
) -> Result<Vec<HashedDifMatch<'data>>, Error> {
    info!(
        "Checking for missing debug information files: {:#?}",
        &objects
    );

    let api = Api::current();
    let missing_checksums = {
        let checksums = objects.iter().map(HashedDifMatch::checksum);
        api.find_missing_dif_checksums(&options.org, &options.project, checksums)?
    };

    let missing = objects
        .into_iter()
        .filter(|sym| missing_checksums.contains(&sym.checksum()))
        .collect();

    info!("Missing debug information files: {:#?}", &missing);
    Ok(missing)
}

/// Compresses the given batch into a ZIP archive.
fn create_batch_archive(difs: &[HashedDifMatch<'_>]) -> Result<TempFile, Error> {
    let total_bytes = difs.iter().map(ItemSize::size).sum();
    let pb = make_byte_progress_bar(total_bytes);
    let tf = TempFile::create()?;

    {
        let mut zip = ZipWriter::new(tf.open()?);

        for symbol in difs {
            zip.start_file(symbol.file_name(), FileOptions::default())?;
            copy_with_progress(&pb, &mut symbol.data(), &mut zip)?;
        }
    }

    pb.finish_and_clear();
    Ok(tf)
}

/// Uploads the given DIFs to the server in batched ZIP archives.
fn upload_in_batches(
    objects: &[HashedDifMatch<'_>],
    options: &DifUpload,
) -> Result<Vec<DebugInfoFile>, Error> {
    let api = Api::current();
    let max_size = Config::current().get_max_dif_archive_size()?;
    let mut dsyms = Vec::new();

    for (i, (batch, _)) in objects.batches(max_size, MAX_CHUNKS).enumerate() {
        println!("\n{}", style(format!("Batch {}", i + 1)).bold());

        println!(
            "{} Compressing {} debug symbol files",
            style(">").dim(),
            style(batch.len()).yellow()
        );
        let archive = create_batch_archive(&batch)?;

        println!("{} Uploading debug symbol files", style(">").dim());
        dsyms.extend(api.upload_dif_archive(&options.org, &options.project, archive.path())?);
    }

    Ok(dsyms)
}

/// Uploads debug info files using the legacy endpoint.
fn upload_difs_batched(options: &DifUpload) -> Result<Vec<DebugInfoFile>, Error> {
    // Search for debug files in the file system and ZIPs
    let found = search_difs(options)?;
    if found.is_empty() {
        println!("{} No debug information files found", style(">").dim());
        return Ok(Default::default());
    }

    // Try to resolve BCSymbolMaps
    let symbol_map = options.symbol_map.as_ref().map(PathBuf::as_path);
    let processed = process_symbol_maps(found, symbol_map)?;

    // Calculate checksums
    let hashed = prepare_difs(processed, HashedDifMatch::from)?;

    // Check which files are missing on the server
    let missing = get_missing_difs(hashed, options)?;
    if missing.is_empty() {
        println!(
            "{} Nothing to upload, all files are on the server",
            style(">").dim()
        );
        println!("{} Nothing to upload", style(">").dim());
        return Ok(Default::default());
    }

    // Upload missing DIFs in batches
    let uploaded = upload_in_batches(&missing, options)?;
    if !uploaded.is_empty() {
        println!("{} File upload complete:\n", style(">").dim());
        for dif in &uploaded {
            println!(
                "  {} ({}; {})",
                style(&dif.id()).dim(),
                &dif.object_name,
                dif.cpu_name
            );
        }
    }

    Ok(uploaded)
}

/// Searches, processes and uploads debug information files (DIFs).
///
/// This struct is created with the `DifUpload::new` function. Then, set
/// search parameters and start the upload via `DifUpload::upload`.
///
/// ```
/// use utils::dif_upload::DifUpload;
///
/// DifUpload::new("org".into(), "project".into())
///     .search_path(".")
///     .upload()?;
/// ```
///
/// The upload tries to perform a chunked upload by requesting the new
/// `chunk-upload/` endpoint. If chunk uploads are disabled or the server does
/// not support them yet, it falls back to the legacy `files/dsyms/` endpoint.
///
/// The uploader will walk the given `paths` in the file system recursively and
/// search for DIFs. If `allow_zips` is not deactivated, it will also open ZIP
/// files and search there.
///
/// By default, all supported object files will be included. To customize this,
/// use the `filter_id`, `filter_kind`, `filter_class` and `filter_extension`
/// methods.
///
/// If `symbol_map` is set and Apple dSYMs with hidden symbols are found, the
/// uploader will first try to locate BCSymbolMaps and generate new dSYMs with
/// resolved symbols.
#[derive(Debug, Default)]
pub struct DifUpload {
    org: String,
    project: String,
    paths: Vec<PathBuf>,
    ids: BTreeSet<DebugId>,
    formats: BTreeSet<FileFormat>,
    features: DifFeatures,
    extensions: BTreeSet<OsString>,
    symbol_map: Option<PathBuf>,
    zips_allowed: bool,
    max_file_size: u64,
    pdbs_allowed: bool,
    include_sources: bool,
}

impl DifUpload {
    /// Creates a new `DifUpload` with default parameters.
    ///
    /// To use it, also add paths using `DifUpload::search_path`. It will scan
    /// the paths and contained ZIPs for all supported object files and upload
    /// them.
    ///
    /// Use `DifUpload::symbol_map` to configure a location of BCSymbolMap files
    /// to resolve hidden symbols in dSYMs obtained from iTunes Connect.
    ///
    /// ```
    /// use utils::dif_upload::DifUpload;
    ///
    /// DifUpload::new("org", "project")
    ///     .search_path(".")
    ///     .upload()?;
    /// ```
    pub fn new(org: String, project: String) -> Self {
        DifUpload {
            org,
            project,
            paths: Vec::new(),
            ids: BTreeSet::new(),
            formats: BTreeSet::new(),
            features: DifFeatures::all(),
            extensions: BTreeSet::new(),
            symbol_map: None,
            zips_allowed: true,
            max_file_size: 2 * 1024 * 1024 * 1024, // 2GB
            pdbs_allowed: false,
            include_sources: false,
        }
    }

    /// Adds a path to search for debug information files.
    pub fn search_path<P>(&mut self, path: P) -> &mut Self
    where
        P: Into<PathBuf>,
    {
        self.paths.push(path.into());
        self
    }

    /// Adds paths to search for debug information files.
    pub fn search_paths<I>(&mut self, paths: I) -> &mut Self
    where
        I: IntoIterator,
        I::Item: Into<PathBuf>,
    {
        for path in paths {
            self.paths.push(path.into())
        }
        self
    }

    /// Add a `DebugId` to filter for.
    ///
    /// By default, all DebugIds will be included.
    pub fn filter_id<I>(&mut self, id: I) -> &mut Self
    where
        I: Into<DebugId>,
    {
        self.ids.insert(id.into());
        self
    }

    /// Add `DebugId`s to filter for.
    ///
    /// By default, all DebugIds will be included. If `ids` is empty, this will
    /// not be changed.
    pub fn filter_ids<I>(&mut self, ids: I) -> &mut Self
    where
        I: IntoIterator,
        I::Item: Into<DebugId>,
    {
        for id in ids {
            self.ids.insert(id.into());
        }
        self
    }

    /// Add an `FileFormat` to filter for.
    ///
    /// By default, all object formats will be included.
    pub fn filter_format(&mut self, format: FileFormat) -> &mut Self {
        self.formats.insert(format);
        self
    }

    /// Add `FileFormat`s to filter for.
    ///
    /// By default, all object formats will be included. If `formats` is empty, this
    /// will not be changed.
    pub fn filter_formats<I>(&mut self, formats: I) -> &mut Self
    where
        I: IntoIterator<Item = FileFormat>,
    {
        self.formats.extend(formats);
        self
    }

    /// Add an `ObjectFeature` to filter for.
    ///
    /// By default, all object features will be included.
    pub fn filter_features(&mut self, features: DifFeatures) -> &mut Self {
        self.features = features;
        self
    }

    /// Add a file extension to filter for.
    ///
    /// By default, all file extensions will be included.
    pub fn filter_extension<S>(&mut self, extension: S) -> &mut Self
    where
        S: Into<OsString>,
    {
        self.extensions.insert(extension.into());
        self
    }

    /// Add a file extension to filter for.
    ///
    /// By default, all file extensions will be included.
    pub fn filter_extensions<I>(&mut self, extensions: I) -> &mut Self
    where
        I: IntoIterator,
        I::Item: Into<OsString>,
    {
        for extension in extensions {
            self.extensions.insert(extension.into());
        }
        self
    }

    /// Set a path containing BCSymbolMaps to resolve hidden symbols in dSYMs
    /// obtained from iTunes Connect. This requires the `dsymutil` command.
    ///
    /// By default, hidden symbol resolution will be skipped.
    pub fn symbol_map<P>(&mut self, path: P) -> Result<&mut Self, Error>
    where
        P: Into<PathBuf>,
    {
        which("dsymutil").map_err(|_| err_msg("Command `dsymutil` not found"))?;
        self.symbol_map = Some(path.into());
        Ok(self)
    }

    /// Set whether opening and searching ZIPs for debug information files is
    /// allowed or not.
    ///
    /// Defaults to `true`.
    pub fn allow_zips(&mut self, allow: bool) -> &mut Self {
        self.zips_allowed = allow;
        self
    }

    /// Set whether source files should be resolved during the scan process and
    /// uploaded as a separate archive.
    ///
    /// Defaults to `false`.
    pub fn include_sources(&mut self, include: bool) -> &mut Self {
        self.include_sources = include;
        self
    }

    /// Performs the search for DIFs and uploads them.
    ///
    /// ```
    /// use utils::dif_upload::DifUpload;
    ///
    /// DifUpload::new("org", "project")
    ///     .search_path(".")
    ///     .upload()?;
    /// ```
    ///
    /// The okay part of the return value is `(files, has_errors)`.  The
    /// latter can be used to indicate a fail state from the upload.
    pub fn upload(&mut self) -> Result<(Vec<DebugInfoFile>, bool), Error> {
        if self.paths.is_empty() {
            println!("{}: No paths were provided.", style("Warning").yellow());
            return Ok(Default::default());
        }

        let api = Api::current();
        if let Some(ref chunk_options) = api.get_chunk_upload_options(&self.org)? {
            if chunk_options.max_file_size > 0 {
                self.max_file_size = chunk_options.max_file_size;
            }

            if chunk_options.supports(ChunkUploadCapability::Pdbs) {
                self.pdbs_allowed = true;
            }

            if self.include_sources && !chunk_options.supports(ChunkUploadCapability::Sources) {
                warn!("Source uploads are not supported by the configured Sentry server");
                self.include_sources = false;
            }

            if chunk_options.supports(ChunkUploadCapability::DebugFiles) {
                return upload_difs_chunked(self, chunk_options);
            }
        }

        if self.include_sources {
            warn!("Source uploads are not supported by the configured Sentry server");
            self.include_sources = false;
        }

        Ok((upload_difs_batched(self)?, false))
    }

    /// Determines if this `DebugId` matches the search criteria.
    fn valid_id(&self, id: DebugId) -> bool {
        self.ids.is_empty() || self.ids.contains(&id)
    }

    /// Determines if this file extension matches the search criteria.
    fn valid_extension(&self, ext: Option<&OsStr>) -> bool {
        self.extensions.is_empty() || ext.map_or(false, |e| self.extensions.contains(e))
    }

    /// Determines if this `FileFormat` matches the search criteria.
    fn valid_format(&self, format: FileFormat) -> bool {
        match format {
            FileFormat::Unknown => false,
            FileFormat::Pdb | FileFormat::Pe if !self.pdbs_allowed => false,
            format => self.formats.is_empty() || self.formats.contains(&format),
        }
    }

    /// Determines if the given `Object` matches the features search criteria.
    fn valid_features(&self, object: &Object<'_>) -> bool {
        self.features.symtab && object.has_symbols()
            || self.features.debug && object.has_debug_info()
            || self.features.unwind && object.has_unwind_info()
    }
}
