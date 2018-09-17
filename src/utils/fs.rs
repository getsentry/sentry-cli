use std::env;
use std::fs;
use std::io;
use std::io::{Read, Seek, SeekFrom};
use std::mem;
use std::path::{Path, PathBuf};

use failure::Error;
use sha1::{Digest, Sha1};
use uuid::{Uuid, UuidVersion};

pub trait SeekRead: Seek + Read {}
impl<T: Seek + Read> SeekRead for T {}

/// Helper for temporary dicts
#[derive(Debug)]
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// Creates a new tempdir
    pub fn new() -> io::Result<TempDir> {
        let mut path = env::temp_dir();
        path.push(
            Uuid::new(UuidVersion::Random)
                .unwrap()
                .hyphenated()
                .to_string(),
        );
        fs::create_dir(&path)?;
        Ok(TempDir { path })
    }

    /// Returns the path to the tempdir
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Helper for temporary file access
#[derive(Debug)]
pub struct TempFile {
    f: Option<fs::File>,
    path: PathBuf,
}

impl TempFile {
    /// Creates a new tempfile.
    pub fn new() -> io::Result<TempFile> {
        let mut path = env::temp_dir();
        path.push(
            Uuid::new(UuidVersion::Random)
                .unwrap()
                .hyphenated()
                .to_string(),
        );

        let f = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)?;

        Ok(TempFile {
            f: Some(f),
            path: path.to_path_buf(),
        })
    }

    /// Assumes ownership over an existing file and moves it to a temp location.
    pub fn take<P: AsRef<Path>>(path: P) -> io::Result<TempFile> {
        let mut destination = env::temp_dir();
        destination.push(
            Uuid::new(UuidVersion::Random)
                .unwrap()
                .hyphenated()
                .to_string(),
        );

        fs::rename(&path, &destination)?;
        let f = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&destination)?;

        Ok(TempFile {
            f: Some(f),
            path: destination,
        })
    }

    /// Opens the tempfile
    pub fn open(&self) -> fs::File {
        let mut f = self.f.as_ref().unwrap().try_clone().unwrap();
        let _ = f.seek(SeekFrom::Start(0));
        f
    }

    /// Returns the path to the tempfile
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the size of the temp file.
    pub fn size(&self) -> io::Result<u64> {
        let mut f = self.open();
        Ok(f.seek(SeekFrom::End(0))?)
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        mem::drop(self.f.take());
        let _ = fs::remove_file(&self.path);
    }
}

/// Checks if a path is writable.
pub fn is_writable<P: AsRef<Path>>(path: P) -> bool {
    fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .map(|_| true)
        .unwrap_or(false)
}

/// Set the mode of a path to 755 if we're on a Unix machine, otherwise
/// don't do anything with the given path.
pub fn set_executable_mode<P: AsRef<Path>>(path: P) -> Result<(), Error> {
    #[cfg(not(windows))]
    fn exec<P: AsRef<Path>>(path: P) -> io::Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let mut perm = fs::metadata(&path)?.permissions();
        perm.set_mode(0o755);
        fs::set_permissions(&path, perm)
    }

    #[cfg(windows)]
    fn exec<P: AsRef<Path>>(_path: P) -> io::Result<()> {
        Ok(())
    }

    exec(path)?;
    Ok(())
}

fn is_zip_file_as_result<R: Read + Seek>(mut rdr: R) -> Result<bool, Error> {
    let mut magic: [u8; 2] = [0; 2];
    rdr.read_exact(&mut magic)?;
    Ok(match &magic {
        b"PK" => true,
        _ => false,
    })
}

/// Checks if a file is a zip file but only returns a bool
pub fn is_zip_file<R: Read + Seek>(rdr: R) -> bool {
    match is_zip_file_as_result(rdr) {
        Ok(val) => val,
        Err(_) => false,
    }
}

/// Returns the SHA1 hash of the given input.
pub fn get_sha1_checksum<R: Read>(rdr: R) -> Result<Digest, Error> {
    let mut sha = Sha1::new();
    let mut buf = [0u8; 16384];
    let mut rdr = io::BufReader::new(rdr);
    loop {
        let read = rdr.read(&mut buf)?;
        if read == 0 {
            break;
        }
        sha.update(&buf[..read]);
    }
    Ok(sha.digest())
}

/// Returns the SHA1 hash for the entire input, as well as each chunk of it. The
/// `chunk_size` must be a power of two.
pub fn get_sha1_checksums(data: &[u8], chunk_size: u64) -> Result<(Digest, Vec<Digest>), Error> {
    if !chunk_size.is_power_of_two() {
        bail!("Chunk size must be a power of two");
    }

    let mut total_sha = Sha1::new();
    let mut chunks = Vec::new();

    for chunk in data.chunks(chunk_size as usize) {
        let mut chunk_sha = Sha1::new();
        chunk_sha.update(chunk);
        total_sha.update(chunk);
        chunks.push(chunk_sha.digest());
    }

    Ok((total_sha.digest(), chunks))
}
