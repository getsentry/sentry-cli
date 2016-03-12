use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use clap::{App, Arg, ArgMatches};
use walkdir::WalkDir;
use which::which;

use CliResult;
use commands::Config;
use macho::is_macho_file;


fn invoke_dsymutil(path: &Path, output_path: &Path) -> CliResult<()> {
    let status = Command::new("dsymutil")
        .arg("-o")
        .arg(output_path)
        .arg("--flat")
        .arg(&path)
        .stderr(Stdio::null())
        .status()?;
    if !status.success() {
        fail!("dsymutil failed to extract symbols");
    }
    Ok(())
}

fn extract_symbols(src: &Path, dst: &Path) -> CliResult<()> {
    for dent_rv in WalkDir::new(src) {
        let dent = dent_rv?;
        let md = dent.metadata()?;
        if !md.is_file() || !is_macho_file(dent.path()) {
            continue;
        }

        let local_name = dent.path().strip_prefix(&src).unwrap();
        let full_path = dst.join(local_name);
        fs::create_dir_all(&full_path.parent().unwrap())?;
        invoke_dsymutil(dent.path(), &full_path)?;

        println!("  {}", local_name.display());
    }
    Ok(())
}


pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b>
{
    app
        .about("extract iOS device support symbols")
        .after_help("This command extracts debug symbols from the iOS Device Support \
                     folder of the system for a given version of IOS and stores them \
                     in a new folder.  This can then later be uploaded into the \
                     global symbol store of a Sentry installation.")
        .arg(Arg::with_name("version")
             .value_name("VERSION")
             .help("The iOS version to convert symbols for.")
             .required(true)
             .index(1))
        .arg(Arg::with_name("path")
             .value_name("PATH")
             .long("path")
             .short("p")
             .help("The path to the iOS Device Support folder if different."))
        .arg(Arg::with_name("output")
             .long("--output")
             .short("-o")
             .help("The output path folder.  If not provided a new folder in the \
                    current folder is used."))
}

pub fn execute<'a>(matches: &ArgMatches<'a>, _config: &Config) -> CliResult<()> {
    let version = matches.value_of("version").unwrap();
    let base_path = matches.value_of("path")
        .map(|p| PathBuf::from(p))
        .unwrap_or_else(|| {
            let mut path = env::home_dir().unwrap();
            path.push("Library/Developer/Xcode/iOS DeviceSupport");
            path
        });
    let source_path = base_path.read_dir()?.filter_map(|ent_rv| {
        if let Ok(ent) = ent_rv {
            let name = ent.file_name();
            if name.to_string_lossy().split_whitespace().nth(0) == Some(version) {
                return Some(ent.path());
            }
        }
        None
    }).next().ok_or_else(|| format!("Could not find symbols for iOS version {} in {}",
                                    version, base_path.display()))?;
    let output = matches.value_of("output")
        .map(|p| PathBuf::from(p))
        .unwrap_or_else(|| {
            let mut path = env::current_dir().unwrap();
            path.push(format!("{}.symbols", version.replace(".", "_")));
            path
        });

    if output.exists() {
        fail!("Cannot proceed because the output path ({}) already exists.",
              output.display());
    }

    if which("dsymutil").is_err() {
        fail!("dsymutil is not installed on this machine but required.");
    }

    println!("Extracting iOS Device Support Symbols");
    println!("  iOS Version: {}", version);
    println!("  Source path: {}", source_path.display());
    println!("  Output path: {}", output.display());

    extract_symbols(&source_path, &output)?;

    println!("All done!");

    Ok(())
}
