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
        println!("cargo:rustc-link-arg=-F");
        println!("cargo:rustc-link-arg=/System/Library/PrivateFrameworks");

        let developer_dir = Command::new("xcode-select")
            .args(["-p"])
            .output()
            .expect("Failed to get developer directory");
        let developer_dir_path = String::from_utf8(developer_dir.stdout)
            .expect("Failed to convert developer directory to UTF-8")
            .trim()
            .to_string();
        println!("cargo:rustc-link-arg=-L");
        println!(
            "cargo:rustc-link-arg={developer_dir_path}/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift/macosx"
        );
    }

    Ok(())
}
