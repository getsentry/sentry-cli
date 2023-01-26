use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("constants.gen.rs");
    let mut f = File::create(dest_path).unwrap();

    let target = env::var("TARGET").unwrap();
    let mut target_bits = target.split('-');

    // https://rust-lang.github.io/rfcs/0131-target-specification.html#detailed-design
    let mut arch = target_bits.next().unwrap();
    let _vendor = target_bits.next();
    let platform = target_bits.next().unwrap();

    if platform == "darwin" && arch == "aarch64" {
        arch = "arm64"; // enforce Darwin naming conventions
    }

    writeln!(f, "/// The platform identifier").ok();
    writeln!(f, "pub const PLATFORM: &str = \"{platform}\";").ok();
    writeln!(f, "/// The CPU architecture identifier").ok();
    writeln!(f, "pub const ARCH: &str = \"{arch}\";").ok();
    writeln!(f, "/// The user agent for sentry events").ok();
    writeln!(f, "pub const USER_AGENT: &str = \"sentry-cli/{arch}\";").ok();
    println!("cargo:rerun-if-changed=build.rs\n");
}
