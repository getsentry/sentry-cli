use std::path::Path;
use std::ffi::OsStr;

use clap::{App, Arg, ArgMatches};
use uuid::Uuid;
use proguard;
use console::style;

use prelude::*;
use config::Config;
use utils::MachoInfo;
use commands::difutil_find::DifType;

enum DifRepr {
    Dsym(MachoInfo),
    Proguard(proguard::MappingView<'static>),
}

impl DifRepr {
    pub fn ty(&self) -> DifType {
        match self {
            &DifRepr::Dsym(..) => DifType::Dsym,
            &DifRepr::Proguard(..) => DifType::Proguard,
        }
    }

    pub fn variants(&self) -> Vec<(Uuid, Option<&'static str>)> {
        match self {
            &DifRepr::Dsym(ref mi) => {
                mi.get_architectures()
                    .into_iter()
                    .map(|(key, value)| (key, Some(value)))
                    .collect()
            }
            &DifRepr::Proguard(ref pg) => {
                vec![(pg.uuid(), None)]
            }
        }
    }

    pub fn is_usable(&self) -> bool {
        match self {
            &DifRepr::Dsym(ref mi) => mi.has_debug_info(),
            &DifRepr::Proguard(ref pg) => pg.has_line_info(),
        }
    }

    pub fn get_problem(&self) -> Option<&str> {
        if self.is_usable() {
            None
        } else {
            Some(match self {
                &DifRepr::Dsym(..) => "missing DWARF debug info",
                &DifRepr::Proguard(..) => "missing line information",
            })
        }
    }
}

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app
        .about("given the path to a debug info file it checks it")
        .arg(Arg::with_name("type")
             .long("type")
             .short("t")
             .value_name("TYPE")
             .possible_values(&["dsym", "proguard"])
             .help("Explicitly sets the type of the debug info file."))
        .arg(Arg::with_name("path")
             .index(1)
             .required(true)
             .help("The path to the debug info file."))
}

pub fn execute<'a>(matches: &ArgMatches<'a>, _config: &Config) -> Result<()> {
    let path = Path::new(matches.value_of("path").unwrap());

    // which types should we consider?
    let ty = matches.value_of("type").map(|t| {
        match t {
            "dsym" => DifType::Dsym,
            "proguard" => DifType::Proguard,
            _ => unreachable!()
        }
    });

    let repr = match ty {
        Some(DifType::Dsym) => DifRepr::Dsym(MachoInfo::open_path(&path)?),
        Some(DifType::Proguard) => DifRepr::Proguard(proguard::MappingView::from_path(&path)?),
        None => {
            if let Ok(mi) = MachoInfo::open_path(&path) {
                DifRepr::Dsym(mi)
            } else {
                match proguard::MappingView::from_path(&path) {
                    Ok(pg) => {
                        if path.extension() == Some(OsStr::new("txt")) ||
                           pg.has_line_info() {
                            DifRepr::Proguard(pg)
                        } else {
                            fail!("invalid debug info file");
                        }
                    }
                    Err(err) => { return Err(err.into()) }
                }
            }
        }
    };

    println!("{}", style("Debug Info File Check").dim().bold());
    println!("  Type: {}", style(repr.ty()).cyan());
    println!("  Contained UUIDs:");
    for (uuid, cpu_type) in repr.variants() {
        if let Some(cpu_type) = cpu_type {
            println!("    > {} ({})", style(uuid).dim(), style(cpu_type).cyan());
        } else {
            println!("    > {}", style(uuid).dim());
        }
    }

    if let Some(prob) = repr.get_problem() {
        println!("  Usable: {} ({})", style("no").red(), prob);
        Err(ErrorKind::QuietExit(1).into())
    } else {
        println!("  Usable: {}", style("yes").green());
        Ok(())
    }
}
