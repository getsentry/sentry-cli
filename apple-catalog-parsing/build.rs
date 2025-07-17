use std::env;
use std::process::Command;

fn main() {
    /// Environment variable to disable Swift sandboxing.
    const SWIFT_DISABLE_SANDBOX: &str = "SWIFT_DISABLE_SANDBOX";

    let target = env::var("TARGET").expect("TARGET is set for build scripts");
    let mut target_bits = target.split('-');

    // https://rust-lang.github.io/rfcs/0131-target-specification.html#detailed-design
    let mut arch = target_bits.next().expect("TARGET triple has an arch");
    let _vendor = target_bits.next();
    let platform = target_bits.next().expect("TARGET triple has a platform");

    if platform != "darwin" {
        return;
    }

    if arch == "aarch64" {
        arch = "arm64"; // enforce Darwin naming conventions
    }

    println!("cargo:rerun-if-changed=native/swift/AssetCatalogParser");

    // Allow swift to be run with `--disable-sandbox` in case cargo has been invoked inside a
    // sandbox already. Nested sandboxes are not allowed on Darwin.
    println!("cargo:rerun-if-env-changed={SWIFT_DISABLE_SANDBOX}");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR is set for build scripts");

    // Compile Swift code
    let status = Command::new("swift")
        .args(
            [
                "build",
                "-c",
                "release",
                "--package-path",
                "native/swift/AssetCatalogParser",
                "--scratch-path",
                &format!("{out_dir}/swift-scratch"),
                "--triple",
                &format!("{arch}-apple-macosx10.12"),
            ]
            .into_iter()
            .chain(
                env::var(SWIFT_DISABLE_SANDBOX)
                    .ok()
                    .filter(|s| s == "1")
                    .map(|_| "--disable-sandbox"),
            ),
        )
        .status()
        .expect("Failed to compile SPM");

    assert!(status.success(), "swift build failed");

    // Create a static library of the Swift and Objective-C code
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

    // Add the new static library to search paths and link to it
    println!("cargo:rustc-link-search=native={out_dir}");
    println!("cargo:rustc-link-lib=static=swiftbridge");

    // Link to CoreUI framework
    println!("cargo:rustc-link-search=framework=/System/Library/PrivateFrameworks");
    println!("cargo:rustc-link-lib=framework=CoreUI");

    // Link to swift macOS support libraries for Swift runtime support on older macOS versions
    let developer_dir = Command::new("xcode-select")
        .args(["-p"])
        .output()
        .expect("Failed to get developer directory, please ensure Xcode is installed.");
    let developer_dir_path = String::from_utf8(developer_dir.stdout)
        .expect("Failed to convert developer directory to UTF-8")
        .trim()
        .to_owned();

    println!(
        "cargo:rustc-link-search={developer_dir_path}/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift/macosx"
    );
}
