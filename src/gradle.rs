use std::fs;
use std::path::Path;

use elementtree::Element;

use prelude::*;


const XMLNS_ANDROID: &'static str = "http://schemas.android.com/apk/res/android";


/// Given the path to an AndroidManifest.xml this parses it and extracts
/// the version code and name.
pub fn get_android_version_from_manifest<P: AsRef<Path>>(path: P)
    -> Result<(u64, String)>
{
    let f = fs::File::open(path)?;
    let manifest = Element::from_reader(f)?;

    let version_code = manifest.get_attr((XMLNS_ANDROID, "versionCode"))
        .ok_or_else(|| Error::from("Could not find version code in android manifest"))?;
    let version_name = manifest.get_attr((XMLNS_ANDROID, "versionName"))
        .ok_or_else(|| Error::from("Could not find version name in android manifest"))?;

    Ok((
        version_code.parse().chain_err(|| "versionCode is not an integer")?,
        version_name.to_string()
    ))
}
