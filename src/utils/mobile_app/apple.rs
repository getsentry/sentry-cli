use std::path::{Path, PathBuf};

use apple_catalog_parsing;
use walkdir::WalkDir;

pub fn handle_asset_catalogs(path: &Path) {
    // Find all asset catalogs
    let cars = find_car_files(path);
    for car in &cars {
        if let Err(e) = apple_catalog_parsing::inspect_asset_catalog(car) {
            eprintln!("Failed to inspect asset catalog {}: {}", car.display(), e);
        }
    }
}

fn find_car_files(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
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
