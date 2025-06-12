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
        println!("cargo:rerun-if-changed=native/swift/AssetCatalogParser");

        env::set_current_dir("native/swift/AssetCatalogParser")
            .expect("Failed to change to AssetCatalogParser directory");

        let target_dir = "../../../target/swift-bridge";

        let _ = std::fs::remove_dir_all(".build");
        let _ = std::fs::remove_file(format!("{}/libswiftbridge.a", target_dir));

        let status = Command::new("swift")
            .args([
                "build",
                "-c",
                "release",
                "--triple",
                &format!("{}-apple-macosx11", arch),
            ])
            .status()
            .expect("Failed to compile SPM");

        assert!(status.success(), "swift build failed");

        let target_dir = "../../../target/swift-bridge";

        std::fs::create_dir_all(target_dir)
            .expect("Failed to create target/swift-bridge directory");

        let status = Command::new("ar")
            .args([
                "crus",
                &format!("{}/libswiftbridge.a", target_dir),
                ".build/release/AssetCatalogParser.build/AssetCatalogReader.swift.o",
                ".build/release/ObjcSupport.build/safeValueForKey.m.o",
            ])
            .status()
            .expect("Failed to create static library");

        assert!(status.success(), "ar failed");

        env::set_current_dir("../../../").expect("Failed to change back to original directory");

        println!("cargo:rustc-link-search=native=target/swift-bridge");
        println!("cargo:rustc-link-lib=static=swiftbridge");

        println!("cargo:rustc-link-arg=-F");
        println!("cargo:rustc-link-arg=/System/Library/PrivateFrameworks");
        println!("cargo:rustc-link-lib=framework=CoreUI");

        let developer_dir = Command::new("xcode-select")
            .args(["-p"])
            .output()
            .expect("Failed to get developer directory");
        let developer_dir_path = String::from_utf8_lossy(&developer_dir.stdout)
            .trim()
            .to_string();
        println!("cargo:rustc-link-arg=-L");
        println!(
            "cargo:rustc-link-arg={}/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift/macosx",
            developer_dir_path
        );
    }

    Ok(())
}
