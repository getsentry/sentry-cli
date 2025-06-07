use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;

fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR is set for build scripts");
    let dest_path = Path::new(&out_dir).join("constants.gen.rs");
    let mut f = File::create(dest_path)?;

    let target = env::var("TARGET").expect("TARGET is set for build scripts");
    let mut target_bits = target.split('-');

    // https://rust-lang.github.io/rfcs/0131-target-specification.html#detailed-design
    let mut arch = target_bits.next().expect("TARGET triple has an arch");
    let _vendor = target_bits.next();
    let platform = target_bits.next().expect("TARGET triple has a platform");

    if platform == "darwin" && arch == "aarch64" {
        arch = "arm64"; // enforce Darwin naming conventions
    }

    writeln!(f, "/// The platform identifier")?;
    writeln!(f, "pub const PLATFORM: &str = \"{platform}\";")?;
    writeln!(f, "/// The CPU architecture identifier")?;
    writeln!(f, "pub const ARCH: &str = \"{arch}\";")?;
    writeln!(f, "/// The user agent for sentry events")?;
    writeln!(f, "pub const USER_AGENT: &str = \"sentry-cli/{arch}\";")?;
    println!("cargo:rerun-if-changed=build.rs\n");

    if platform == "darwin" {
        println!("cargo:rerun-if-changed=src/apple/AssetCatalogReader.swift");
        println!("cargo:rerun-if-changed=src/apple/safeValueForKey.h");
        println!("cargo:rerun-if-changed=src/apple/safeValueForKey.m");
        // Compile Objective-C
        let status = Command::new("clang")
            .args([
                "src/apple/safeValueForKey.m",
                "-c",
                "-o",
                "safeValueForKey.o",
                "-fobjc-arc",
            ])
            .status()
            .expect("Failed to compile Objective-C");

        assert!(status.success(), "clang failed");

        // Compile Swift and link ObjC
        let status = Command::new("swiftc")
            .args([
                "-emit-library",
                "-static",
                "-o",
                "libswiftbridge.a",
                "src/apple/AssetCatalogReader.swift",
                "safeValueForKey.o",
                "-import-objc-header",
                "src/apple/safeValueForKey.h",
            ])
            .status()
            .expect("Failed to compile Swift");

        assert!(status.success(), "swiftc failed");

        println!("cargo:rustc-link-search=native=.");
        println!("cargo:rustc-link-lib=static=swiftbridge");

        println!("cargo:rustc-link-arg=-F");
        println!("cargo:rustc-link-arg=/System/Library/PrivateFrameworks");
        println!("cargo:rustc-link-lib=framework=CoreUI");
    }

    Ok(())
}
