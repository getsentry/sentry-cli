//! Searches, processes and uploads debug information files (DIFs). See
//! `DifUpload` for more information.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::{self, File};
use std::iter::IntoIterator;
use std::io::{Read, Seek, SeekFrom, Write};
use std::ops::Deref;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::rc::Rc;
use std::slice::{Chunks, Iter};
use std::str;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use clap::ArgMatches;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use sha1::Digest;
use symbolic_common::{ByteView, ObjectClass, ObjectKind};
use symbolic_debuginfo::{FatObject, ObjectId};
use walkdir::WalkDir;
use zip::{ZipArchive, ZipWriter};
use zip::write::FileOptions;

use api::{self, Api, ChunkUploadOptions};
use config::Config;
use errors::Result;
use utils::{copy_with_progress, make_byte_progress_bar, TempDir, TempFile, get_sha1_checksum,
            get_sha1_checksums};
use utils::batch::{BatchedSliceExt, ItemSize};
use utils::dif::has_hidden_symbols;

/// Fallback maximum number of chunks in a batch for the legacy upload.
static MAX_CHUNKS: u64 = 64;

/// A single chunk of a debug information file returned by
/// `ChunkedDifMatch::chunks`. It carries the binary data slice and a SHA1
/// checksum of that data.
///
/// `DifChunk` implements AsRef<(Digest, &[u8])> so that it can be easily
/// transformed into a vector or map.
#[derive(Debug)]
struct DifChunk<'data>((Digest, &'data [u8]));

impl<'data> AsRef<(Digest, &'data [u8])> for DifChunk<'data> {
    fn as_ref(&self) -> &(Digest, &'data [u8]) {
        &self.0
    }
}

impl<'data> ItemSize for DifChunk<'data> {
    fn size(&self) -> u64 {
        (self.0).1.len() as u64
    }
}

/// An iterator over chunks of data in a `ChunkedDifMatch` object.
///
/// This struct is returned by `ChunkedDifMatch::chunks`.
struct DifChunks<'a> {
    checksums: Iter<'a, Digest>,
    iter: Chunks<'a, u8>,
}

impl<'a> Iterator for DifChunks<'a> {
    type Item = DifChunk<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.checksums.next(), self.iter.next()) {
            (Some(checksum), Some(data)) => Some(DifChunk((*checksum, data))),
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
    fat: Rc<FatObject<'data>>,
    name: String,
    attachments: Option<BTreeMap<String, ByteView<'static>>>,
}

impl<'data> DifMatch<'data> {
    /// Moves the specified temporary debug file to a safe location and assumes
    /// ownership. The file will be deleted in the file system when this
    /// `DifMatch` is dropped.
    fn take_temp<P, S>(path: P, name: S) -> Result<DifMatch<'static>>
    where
        P: AsRef<Path>,
        S: Into<String>,
    {
        let temp_file = TempFile::take(path)?;
        let buffer = ByteView::from_path(temp_file.path())?;

        Ok(DifMatch {
            _backing: Some(DifBacking::Temp(temp_file)),
            fat: Rc::new(FatObject::parse(buffer)?),
            name: name.into(),
            attachments: None,
        })
    }

    /// Returns the parsed `FatObject` of this DIF.
    pub fn fat(&self) -> &FatObject {
        &self.fat
    }

    /// Returns the raw binary data of this DIF.
    pub fn data(&self) -> &[u8] {
        self.fat().as_bytes()
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
            .and_then(|name| name.to_str())
            .unwrap_or("Generic")
    }

    /// Returns attachments of this DIF, if any.
    pub fn attachments(&self) -> Option<&BTreeMap<String, ByteView>> {
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

        has_hidden_symbols(self.fat()).unwrap_or(false)
    }
}

impl<'data> fmt::Debug for DifMatch<'data> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("DifMatch")
            .field("fat", &self.fat)
            .field("object_count", &self.name)
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
    fn from(inner: DifMatch) -> Result<HashedDifMatch> {
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
    pub fn from(inner: DifMatch, chunk_size: u64) -> Result<ChunkedDifMatch> {
        let (checksum, chunks) = get_sha1_checksums(inner.data(), chunk_size)?;
        Ok(ChunkedDifMatch {
            inner: HashedDifMatch { inner, checksum },
            chunks: chunks,
            chunk_size: chunk_size,
        })
    }

    /// Returns an iterator over all chunk checksums.
    pub fn checksums(&self) -> Iter<Digest> {
        self.chunks.iter()
    }

    /// Returns an iterator over all `DifChunk`s.
    pub fn chunks(&self) -> DifChunks {
        DifChunks {
            checksums: self.checksums(),
            iter: self.data().chunks(self.chunk_size as usize),
        }
    }

    /// Creates a tuple which can be collected into a `api::AssembleDifsRequest`.
    pub fn to_assemble(&self) -> (Digest, api::ChunkedDifRequest) {
        (
            self.checksum(),
            api::ChunkedDifRequest {
                name: self.file_name(),
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
    Zip(&'a mut ZipArchive<File>, &'a str),
}

impl<'a> DifSource<'a> {
    /// Resolves a file relative to the directory of `base`, stripping of the
    /// file name.
    fn get_relative_fs(base: &Path, path: &Path) -> Option<ByteView<'static>> {
        // Use parent() to get to the directory and then move relative from
        // there. ByteView will internally cannonicalize the path and resolve
        // symlinks.
        base.parent()
            .and_then(|p| ByteView::from_path(p.join(path)).ok())
    }

    /// Extracts a file relative to the directory of `name`, stripping of the
    /// file name.
    fn get_relative_zip(
        zip: &mut ZipArchive<File>,
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
            .and_then(|f| ByteView::from_reader(f).ok())
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
type MissingDifsInfo<'data> = (Vec<&'data ChunkedDifMatch<'data>>, Vec<DifChunk<'data>>);

/// Verifies that the given path contains a ZIP file and opens it.
fn try_open_zip<P>(path: P) -> Result<Option<ZipArchive<File>>>
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
        b"PK" => Some(ZipArchive::new(file)?),
        _ => None,
    })
}

/// Searches the given ZIP for potential DIFs and passes them to the callback.
///
/// To avoid unnecessary file operations, the file extension is already checked
/// for every entry before opening it.
///
/// This function will not recurse into ZIPs contained in this ZIP.
fn walk_difs_zip<F>(mut zip: ZipArchive<File>, options: &DifUpload, mut func: F) -> Result<()>
where
    F: FnMut(DifSource, String, ByteView<'static>) -> Result<()>,
{
    for index in 0..zip.len() {
        let (name, buffer) = {
            let zip_file = zip.by_index(index)?;
            let name = zip_file.name().to_string();

            if !options.valid_extension(Path::new(&name).extension()) {
                continue;
            }

            (name, ByteView::from_reader(zip_file)?)
        };

        func(DifSource::Zip(&mut zip, &name), name.clone(), buffer)?;
    }

    Ok(())
}

/// Recursively searches the given directory for potential DIFs and passes them
/// to the callback.
///
/// If `DifUpload::allow_zips` is set, then this function will attempt to open
/// the ZIP and search it for DIFs as well, however not recursing further into
/// nested ZIPs.
///
/// To avoid unnecessary file operations, the file extension is already checked
/// for every entry before opening it.
fn walk_difs_directory<F, P>(directory: P, options: &DifUpload, mut func: F) -> Result<()>
where
    P: AsRef<Path>,
    F: FnMut(DifSource, String, ByteView<'static>) -> Result<()>,
{
    for entry in WalkDir::new(&directory).into_iter().filter_map(|e| e.ok()) {
        if !entry.metadata()?.is_file() {
            // Walkdir recurses automatically into folders
            continue;
        }

        let path = entry.path();
        if let Some(zip) = try_open_zip(path)? {
            walk_difs_zip(zip, options, &mut func)?;
            continue;
        }

        if !options.valid_extension(path.extension()) {
            continue;
        }

        let buffer = ByteView::from_path(path)?;
        let name = path.strip_prefix(&directory)
            .unwrap()
            .to_string_lossy()
            .into_owned();

        func(DifSource::FileSystem(path), name, buffer)?;
    }

    Ok(())
}

/// Searches for mapping PLists next to the given `source`. It returns a mapping
/// of Plist name to owning buffer of the file's contents. This function should
/// only be called for dSYMs.
fn find_uuid_plists(
    fat: &FatObject,
    source: &mut DifSource,
) -> Option<BTreeMap<String, ByteView<'static>>> {
    let mut plists = BTreeMap::new();

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
    for id in fat.objects().filter_map(|o| o.ok()).filter_map(|o| o.id()) {
        let plist_name = format!("{}.plist", id.uuid().to_string().to_uppercase());
        if let Some(plist) = source.get_relative(format!("../{}", &plist_name)) {
            plists.insert(plist_name, plist);
        }
    }

    // In case there are no such plists (e.g. for a local build), return None
    // instead of an empty map. This allows to exit earlier when processing
    // the `DifMatches`.
    if plists.is_empty() {
        None
    } else {
        Some(plists)
    }
}

/// Searches matching debug information files.
fn search_difs(options: &DifUpload) -> Result<Vec<DifMatch<'static>>> {
    let progress_style = ProgressStyle::default_spinner().template(
        "{spinner} Searching for debug symbol files...\
         \n  found {prefix:.yellow} {msg:.dim}",
    );

    let progress = ProgressBar::new_spinner();
    progress.enable_steady_tick(100);
    progress.set_style(progress_style);

    let mut collected = Vec::new();
    let mut found_ids = BTreeSet::new();
    for base_path in &options.paths {
        walk_difs_directory(base_path, options, |mut source, name, buffer| {
            progress.set_message(&name);

            // Try to parse a potential object file. If this is not possible,
            // then we're not dealing with an object file, thus silently
            // skipping it.
            let kind = FatObject::peek(&buffer).ok();
            if !kind.map_or(false, |k| options.valid_kind(k)) {
                return Ok(());
            }

            // This is a hack to allow iteration through the FatObject while
            // also moving it into the DifMatch, in case it contains a matching
            // object.
            let mut symbol_added = false;
            let fat = Rc::new(FatObject::parse(buffer)?);
            for object in fat.objects() {
                let object = object?;

                // If an object object class was specified, this will skip all
                // other objects. Usually, the user will search for
                // `ObjectClass::Debug` (i.e. dSYM or Breakpad) only, but in
                // future we might want to upload other files (e.g. executables),
                // too.
                if !options.valid_class(object.class()) {
                    continue;
                }

                // Objects without UUID will be skipped altogether. While frames
                // during symbolication might be lacking debug identifiers,
                // Sentry requires object files to have one during upload.
                let id = match object.id() {
                    Some(id) => id,
                    None => continue,
                };

                // Make sure we haven't converted this object already.
                if !options.valid_id(id) || found_ids.contains(&id) {
                    continue;
                }

                // We only collect the DifMatch once per FatObject but continue
                // to iterate so that we capture all matching UUIDs. This will
                // allow us to skip all UUIDs of objects that do not match the
                // search criteria.
                found_ids.insert(id);
                if symbol_added {
                    continue;
                }

                // Invoke logic to retrieve attachments specific to the kind
                // of object file. These are used for processing. Since only
                // dSYMs equire processing currently, all other kinds are
                // skipped.
                let attachments = match fat.kind() {
                    ObjectKind::MachO => find_uuid_plists(&fat, &mut source),
                    _ => None,
                };

                symbol_added = true;
                collected.push(DifMatch {
                    _backing: None,
                    fat: fat.clone(),
                    name: name.clone(),
                    attachments: attachments,
                });

                progress.set_prefix(&collected.len().to_string());
            }

            Ok(())
        })?;
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
fn resolve_hidden_symbols<'a>(dif: DifMatch<'a>, symbol_map: &Path) -> Result<DifMatch<'a>> {
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
    let temp_dir = TempDir::new()?;
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
            return Err(format!("Could not resolve BCSymbolMaps: {}", error).into());
        } else {
            return Err("Could not resolve BCSymbolMaps due to an unknown error".into());
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
fn prepare_difs<'data, F, T>(items: Vec<DifMatch<'data>>, mut func: F) -> Result<Vec<T>>
where
    F: FnMut(DifMatch<'data>) -> Result<T>,
{
    let progress_style = ProgressStyle::default_bar().template(
        "{prefix:.dim} Preparing for upload... {msg:.dim}\
         \n  {wide_bar}  {pos}/{len}",
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
) -> Result<Vec<DifMatch<'a>>> {
    let (with_hidden, mut without_hidden): (Vec<_>, _) = difs.into_iter()
        .partition(|dif| dif.needs_symbol_map());

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
         \n  {wide_bar}  {pos}/{len}",
    );

    let progress = ProgressBar::new(with_hidden.len() as u64);
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

/// Calls the assemble endpoint and returns the state for every `DifMatch` along
/// with info on missing chunks.
///
/// The returned value containes separate vectors for incomplete DIFs and
/// missing chunks for convenience.
fn try_assemble_difs<'data>(
    api: &Api,
    difs: &'data Vec<ChunkedDifMatch<'data>>,
    options: &DifUpload,
) -> Result<Option<MissingDifsInfo<'data>>> {
    let request = difs.iter().map(ChunkedDifMatch::to_assemble).collect();
    let response = api.assemble_difs(&options.org, &options.project, &request)?;

    // We map all DIFs by their checksum, so we can access them faster when
    // iterating through the server response below. Since the caller will invoke
    // this function multiple times (most likely twice), this operation is
    // performed twice with the same data. While this is redundant, it is also
    // fast enough and keeping it here makes the `try_assemble_difs` interface
    // nicer.
    let difs_by_checksum: BTreeMap<_, _> = difs.iter().map(|m| (m.checksum, m)).collect();

    let mut difs = Vec::new();
    let mut chunks = Vec::new();
    for (checksum, ref file_response) in response {
        let chunked_match = *difs_by_checksum
            .get(&checksum)
            .ok_or("Server returned unexpected checksum")?;

        match file_response.state {
            api::ChunkedFileState::Error => {
                // One of the files could not be uploaded properly and resulted
                // in an error. We might still want to wait for all other files,
                // however, so we ignore this file for now.
            }
            api::ChunkedFileState::NotFound => {
                // Assembling for one of the files has not started because some
                // (or all) of its chunks have not been found. We report its
                // missing chunks to the caller and then continue. The caller
                // will have to call `try_assemble_difs` again after uploading
                // them.
                let mut missing_chunks = chunked_match
                    .chunks()
                    .filter(|&DifChunk((c, _))| file_response.missing_chunks.contains(&c))
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
                // This file is currently assembling or has already finished. No
                // action required anymore. The caller will have to poll this
                // file later until it either resolves or errors.
            }
        }
    }

    if chunks.is_empty() {
        Ok(None)
    } else {
        Ok(Some((difs, chunks)))
    }
}

/// Uploads chunks specified in `missing_info` in batches. The batch size is
/// controlled by `chunk_options`.
///
/// This function blocks until all chunks have been uploaded.
fn upload_missing_chunks(
    api: &Api,
    missing_info: &MissingDifsInfo,
    chunk_options: &ChunkUploadOptions,
) -> Result<()> {
    let progress_style = ProgressStyle::default_bar().template(
        "{prefix:.dim} Uploading {msg:.yellow} missing debug information files...\
         \n  {wide_bar}  {bytes}/{total_bytes} ({eta})",
    );

    // Chunks are uploaded in batches, but the progress bar is shared between
    // multiple requests to simulate one continuous upload to the user. Since we
    // have to embed the progress bar into a ProgressBarMode and move it into
    // `Api::upload_chunks`, the progress bar is created in an Arc.
    let &(ref difs, ref chunks) = missing_info;
    let total = chunks
        .iter()
        .map(|&DifChunk((_, data))| data.len() as u64)
        .sum();

    let progress = Arc::new(ProgressBar::new(total));
    progress.set_style(progress_style);
    progress.set_prefix(">");
    progress.set_message(&difs.len().to_string());

    // Since each upload is separate inside `Api::upload_chunks`, we need to
    // keep track of the progress and pass it as offset into
    // `ProgressBarMode::Shared`. Each batch aggregates objects until it exceeds
    // the maximum size configured in ChunkUploadOptions.
    let mut base = 0;
    for (batch, size) in chunks.batches(chunk_options.max_size, chunk_options.max_chunks) {
        let mode = api::ProgressBarMode::Shared((progress.clone(), size, base));
        api.upload_chunks(&chunk_options.url, batch, mode)?;
        base += size;
    }

    progress.finish_and_clear();
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

/// Polls the assemble endpoint until all DIFs have either completed or errored
/// and prints a summary in the end.
fn poll_dif_assemble(api: &Api, difs: Vec<&ChunkedDifMatch>, options: &DifUpload) -> Result<()> {
    let progress_style = ProgressStyle::default_bar().template(
        "{prefix:.dim} Processing files...\
         \n  {wide_bar}  {pos}/{len}",
    );

    let progress = ProgressBar::new(difs.len() as u64);
    progress.set_style(progress_style);
    progress.set_prefix(">");

    let request = difs.iter().map(|d| d.to_assemble()).collect();
    let response = loop {
        let response = api.assemble_difs(&options.org, &options.project, &request)?;
        let pending = response.iter().filter(|&(_, r)| r.state.pending()).count();
        progress.set_position((difs.len() - pending) as u64);

        if pending == 0 {
            break response;
        }

        thread::sleep(Duration::from_millis(5000));
    };

    progress.finish_and_clear();
    println!("{} File processing complete:", style(">").dim());

    let difs_by_checksum: BTreeMap<_, _> = difs.iter().map(|m| (m.checksum, m)).collect();

    for (checksum, r) in response {
        let chunked_match = difs_by_checksum
            .get(&checksum)
            .ok_or("Server returned unexpected checksum")?;

        let state = match r.state.ok() {
            true => style("OK").green(),
            false => style("ERROR").red(),
        };

        println!("  {:>5} {}", state, chunked_match.file_name());
    }

    Ok(())
}

/// Uploads debug info files using the chunk-upload endpoint.
fn upload_difs_chunked(
    api: &Api,
    options: &DifUpload,
    chunk_options: &ChunkUploadOptions,
) -> Result<()> {
    // Search for debug files in the file system and ZIPs
    let found = search_difs(options)?;
    if found.is_empty() {
        println!(
            "{} No debug debug information files found",
            style(">").dim()
        );
        return Ok(());
    }

    // Try to resolve BCSymbolMaps
    let symbol_map = options.symbol_map.as_ref().map(PathBuf::as_path);
    let processed = process_symbol_maps(found, symbol_map)?;

    // Calculate checksums and chunks
    let chunked = prepare_difs(processed, |m| {
        ChunkedDifMatch::from(m, chunk_options.chunk_size)
    })?;

    // Upload until all chunks are present on the server
    let mut initially_missing = None;
    while let Some(missing_info) = try_assemble_difs(api, &chunked, options)? {
        upload_missing_chunks(api, &missing_info, chunk_options)?;
        initially_missing.get_or_insert(missing_info);
    }

    // Only if DIFs were missing, poll until assembling is complete
    if let Some((missing, _)) = initially_missing {
        poll_dif_assemble(api, missing, options)?;
    } else {
        println!(
            "{} Nothing to upload, all files are on the server",
            style(">").dim()
        );
    }

    Ok(())
}

/// Returns debug files missing on the server.
fn get_missing_difs<'data>(
    api: &Api,
    objects: Vec<HashedDifMatch<'data>>,
    options: &DifUpload,
) -> Result<Vec<HashedDifMatch<'data>>> {
    info!(
        "Checking for missing debug information files: {:#?}",
        &objects
    );

    let missing_checksums = {
        let checksums = objects.iter().map(|s| s.checksum());
        api.find_missing_dsym_checksums(&options.org, &options.project, checksums)?
    };

    let missing = objects
        .into_iter()
        .filter(|sym| missing_checksums.contains(&sym.checksum()))
        .collect();

    info!("Missing debug information files: {:#?}", &missing);
    Ok(missing)
}

/// Compresses the given batch into a ZIP archive.
fn create_batch_archive(difs: &[HashedDifMatch]) -> Result<TempFile> {
    let total_bytes = difs.iter().map(|sym| sym.size()).sum();
    let pb = make_byte_progress_bar(total_bytes);
    let tf = TempFile::new()?;
    let mut zip = ZipWriter::new(tf.open());

    for ref symbol in difs {
        zip.start_file(symbol.file_name(), FileOptions::default())?;
        copy_with_progress(&pb, &mut symbol.data(), &mut zip)?;
    }

    pb.finish_and_clear();
    Ok(tf)
}

/// Uploads the given DIFs to the server in batched ZIP archives.
fn upload_in_batches(
    api: &Api,
    objects: Vec<HashedDifMatch>,
    options: &DifUpload,
) -> Result<Vec<api::DSymFile>> {
    let max_size = Config::get_current().get_max_dsym_upload_size()?;
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
        dsyms.extend(api.upload_dsyms(&options.org, &options.project, archive.path())?);
    }

    Ok(dsyms)
}

/// Uploads debug info files using the legacy endpoint.
fn upload_difs_batched(api: &Api, options: &DifUpload) -> Result<()> {
    // Search for debug files in the file system and ZIPs
    let found = search_difs(options)?;
    if found.is_empty() {
        println!("{} No debug information files found", style(">").dim());
        return Ok(());
    }

    // Try to resolve BCSymbolMaps
    let symbol_map = options.symbol_map.as_ref().map(PathBuf::as_path);
    let processed = process_symbol_maps(found, symbol_map)?;

    // Calculate checksums
    let hashed = prepare_difs(processed, |m| HashedDifMatch::from(m))?;

    // Check which files are missing on the server
    let missing = get_missing_difs(api, hashed, options)?;
    if missing.len() == 0 {
        println!(
            "{} Nothing to upload, all files are on the server",
            style(">").dim()
        );
        println!("{} Nothing to upload", style(">").dim());
        return Ok(());
    }

    // Upload missing DIFs in batches
    let uploaded = upload_in_batches(api, missing, options)?;
    if uploaded.len() > 0 {
        println!("Newly uploaded debug information files:");
        for dif in &uploaded {
            println!(
                "  {} ({}; {})",
                style(&dif.uuid).dim(),
                &dif.object_name,
                dif.cpu_name
            );
        }
    }

    Ok(())
}

/// Searches, processes and uploads debug information files (DIFs).
///
/// This struct is created with the `DifUpload::from_cli` from command line
/// arguments. Then, an upload can either be started via `DifUpload::upload` or
/// `DifUpload::upload_with`.
///
/// ```
/// use clap::App;
/// use utils::dif_upload::DifUpload;
///
/// let matches = App::new("My App")
///     // Add arguments here
///     .get_matches();
///
/// DifUpload::from_cli(&matches)?
///     .upload()?;
/// ```
///
/// Alternatively, create a new instance with `DifUpload::new` and set
/// parameters manually.
///
/// ```
/// use utils::dif_upload::DifUpload;
///
/// DifUpload::new("org", "project")
///     .add_path(".")
///     .upload()?;
/// ```
///
/// The upload tries to perform a chunked upload by requesting the new
/// `chunk-upload/` endpoint. If chunk uploads are disabled or the server does
/// not support them yet, it falls back to the legacy `files/dsyms/` endpoint.
#[derive(Debug, Default)]
pub struct DifUpload {
    org: String,
    project: String,
    paths: Vec<PathBuf>,
    ids: BTreeSet<ObjectId>,
    kinds: BTreeSet<ObjectKind>,
    classes: BTreeSet<ObjectClass>,
    extensions: BTreeSet<OsString>,
    symbol_map: Option<PathBuf>,
    zips_allowed: bool,
}

impl DifUpload {
    /// Creates a new `DifUpload` with default parameters.
    pub fn new(org: String, project: String) -> DifUpload {
        DifUpload {
            org: org,
            project: project,
            paths: Default::default(),
            ids: Default::default(),
            kinds: Default::default(),
            classes: Default::default(),
            extensions: Default::default(),
            symbol_map: None,
            zips_allowed: true,
        }
    }

    /// Creates an `DifUpload` object from command line arguments. Supported
    /// arguments are:
    ///
    ///  - `"--org"`: The organization slug
    ///  - `"--project"`: The project slug
    ///  - `"--id"`: Missing Debug Identifiers (`ObjectId`) to search for
    ///  - `"--uuid"`: Missing UUIDs to search for (converted to `ObjectId`s)
    ///  - `"--type"`: Specify the type of files ("dsym", "breakpad", "elf")
    ///  - `"--no-executables"`: Exclude executables and search for symbols only
    ///  - `"--no-debug"`: Exclude symbols and search for executables only
    ///  - `"--symbol-maps"`: Path to a BCSymbolMap folder
    ///  - `"--no-zips"`: Do not recurse into ZIPs
    ///  - `<...>`: Paths to search for debug information files
    ///
    /// ```
    /// use clap::App;
    /// use utils::dif_upload::DifUpload;
    ///
    /// let matches = App::new("My App")
    ///     // Add arguments here
    ///     .get_matches();
    ///
    /// let upload = DifUpload::from_cli(&matches)?;
    /// println!("{:#?}", upload);
    /// ```
    pub fn from_cli(_matches: &ArgMatches) -> Result<DifUpload> {
        unimplemented!();
    }

    /// Performs the search for DIFs and uploads them using the given `Api`.
    ///
    /// ```
    /// let api = Api::new();
    /// let matches = App::new("My App").get_matches();
    /// DifUpload::from_cli(&matches)?.upload_with(&api)?;
    /// ```
    pub fn upload_with(self, api: &Api) -> Result<()> {
        if let Some(ref chunk_options) = api.get_chunk_upload_options(&self.org)? {
            upload_difs_chunked(api, &self, chunk_options)
        } else {
            upload_difs_batched(api, &self)
        }
    }

    /// Performs the search for DIFs and uploads them using a new `Api` instance.
    ///
    /// ```
    /// let matches = App::new("My App").get_matches();
    /// DifUpload::from_cli(&matches)?.upload()?;
    /// ```
    pub fn upload(self) -> Result<()> {
        self.upload_with(&Api::new())
    }

    /// Determines if this `ObjectId` matches the search criteria.
    fn valid_id(&self, id: ObjectId) -> bool {
        self.ids.is_empty() || self.ids.contains(&id)
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
}
