use std::env;
use std::error::Error;
use std::process::Command;

fn main() -> Result<(), Box<dyn Error>> {
    let target = env::var("TARGET").expect("TARGET is set for build scripts");
    let mut target_bits = target.split('-');

    // https://rust-lang.github.io/rfcs/0131-target-specification.html#detailed-design
    let mut arch = target_bits.next().expect("TARGET triple has an arch");
    let _vendor = target_bits.next();
    let platform = target_bits.next().expect("TARGET triple has a platform");

    if platform != "darwin" {
        return Ok(());
    }

    if arch == "aarch64" {
        arch = "arm64"; // enforce Darwin naming conventions
    }

    println!("cargo:rerun-if-changed=native/swift/AssetCatalogParser");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR is set for build scripts");

    let status = Command::new("swift")
        .args([
            "build",
            "-c",
            "release",
            "--package-path",
            "native/swift/AssetCatalogParser",
            "--scratch-path",
            &format!("{out_dir}/swift-scratch"),
            "--triple",
            &format!("{arch}-apple-macosx11"),
        ])
        .status()
        .expect("Failed to compile SPM");

    assert!(status.success(), "swift build failed");

    let status = Command::new("ar")
        .args([
            "crus",
            &format!("{out_dir}/libswiftbridge.a"),
            &format!(
                "{out_dir}/swift-scratch/release/AssetCatalogParser.build/AssetCatalogReader.swift.o"
            ),
            &format!(
                "{out_dir}/swift-scratch/release/ObjcSupport.build/safeValueForKey.m.o",
            ),
        ])
        .status()
        .expect("Failed to create static library");

    assert!(status.success(), "ar failed");

    println!("cargo:rustc-link-search=native={out_dir}");
    println!("cargo:rustc-link-lib=static=swiftbridge");
    println!("cargo:rustc-link-lib=framework=CoreUI");

    Ok(())
}
