use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

extern "C" {
    fn swift_inspect_asset_catalog(msg: *const std::os::raw::c_char);
}

// This calls out to Swift code that uses Apple APIs to convert the contents
// of an asset catalog into a format that can be parsed by the
// size analysis backend. It enables main size analysis features such
// as duplicate image detection, xray, and image optimization insights.
pub fn inspect_asset_catalog<P: AsRef<Path>>(path: P) {
    let c_string = CString::new(path.as_ref().as_os_str().as_bytes()).expect("CString::new failed");
    unsafe {
        swift_inspect_asset_catalog(c_string.as_ptr());
    }
}
