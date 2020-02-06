use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("constants.gen.rs");
    let mut f = File::create(&dest_path).unwrap();

    let target = env::var("TARGET").unwrap();
    let mut target_bits = target.split('-');
    let arch = target_bits.next().unwrap();
    target_bits.next();
    let platform = target_bits.next().unwrap();

    writeln!(f, "/// The platform identifier").ok();
    writeln!(f, "pub const PLATFORM: &str = \"{}\";", platform).ok();
    writeln!(f, "/// The CPU architecture identifier").ok();
    writeln!(f, "pub const ARCH: &str = \"{}\";", arch).ok();
    writeln!(f, "/// The user agent for sentry events").ok();
    writeln!(f, "pub const USER_AGENT: &str = \"sentry-cli/{}\";", arch).ok();
    println!("cargo:rerun-if-changed=build.rs\n");
}
