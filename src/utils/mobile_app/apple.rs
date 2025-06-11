#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::path::{Path, PathBuf};
    use walkdir::WalkDir;

    extern "C" {
        fn swift_inspect_asset_catalog(msg: *const std::os::raw::c_char);
    }

    pub fn handle_asset_catalogs(path: &Path) {
        // Find all asset catalogs
        let cars = find_car_files(path);
        for car in &cars {
            inspect_asset_catalog(car);
        }
    }

    fn inspect_asset_catalog<P: AsRef<Path>>(path: P) {
        let c_string =
            CString::new(path.as_ref().as_os_str().as_bytes()).expect("CString::new failed");
        unsafe {
            swift_inspect_asset_catalog(c_string.as_ptr());
        }
    }

    fn find_car_files(root: &Path) -> Vec<PathBuf> {
        WalkDir::new(root)
            .into_iter()
            .filter_map(Result::ok) // discard I/O errors
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
}

#[cfg(not(target_os = "macos"))]
mod macos {
    use std::path::Path;

    pub fn handle_asset_catalogs(_path: &Path) {
        // No-op for non-macOS platforms
    }
}

pub use macos::handle_asset_catalogs;
