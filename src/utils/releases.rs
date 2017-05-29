use utils::vcs;

use prelude::*;

/// Detects the release name for the current working directory.
pub fn detect_release_name() -> Result<String>
{
    Ok(vcs::find_head()?)
}
