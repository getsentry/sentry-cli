use std::env;
use std::fs;
use std::io;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use failure::{bail, Error};
use sha1::{Digest, Sha1};
use uuid::Uuid;

pub trait SeekRead: Seek + Read {}
impl<T: Seek + Read> SeekRead for T {}

/// Helper for temporary dicts
#[derive(Debug)]
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// Creates a new tempdir
    pub fn create() -> io::Result<Self> {
        let mut path = env::temp_dir();
        path.push(Uuid::new_v4().to_hyphenated_ref().to_string());
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
    path: PathBuf,
}

impl TempFile {
    /// Creates a new tempfile.
    pub fn create() -> io::Result<Self> {
        let mut path = env::temp_dir();
        path.push(Uuid::new_v4().to_hyphenated_ref().to_string());

        let tf = TempFile { path };
        tf.open()?;
        Ok(tf)
    }

    /// Assumes ownership over an existing file and moves it to a temp location.
    pub fn take<P: AsRef<Path>>(path: P) -> io::Result<TempFile> {
        let mut destination = env::temp_dir();
        destination.push(Uuid::new_v4().to_hyphenated_ref().to_string());

        fs::rename(&path, &destination)?;
        Ok(TempFile { path: destination })
    }

    /// Opens the tempfile at the beginning.
    pub fn open(&self) -> io::Result<fs::File> {
        let mut f = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&self.path)?;

        f.seek(SeekFrom::Start(0)).ok();
        Ok(f)
    }

    /// Returns the path to the tempfile.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the size of the temp file.
    pub fn size(&self) -> io::Result<u64> {
        self.open()?.seek(SeekFrom::End(0))
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        fs::remove_file(&self.path).ok();
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
