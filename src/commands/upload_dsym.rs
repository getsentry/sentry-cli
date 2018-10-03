//! Implements a command for uploading dSYM files.
use clap::{App, AppSettings, Arg, ArgMatches};
use failure::Error;

use commands::upload_dif;
use utils::args::{validate_uuid, ArgExt};

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("DEPRECATED: Upload Mac debug symbols to a project.")
        .setting(AppSettings::Hidden)
        .org_project_args()
        .arg(
            Arg::with_name("paths")
                .value_name("PATH")
                .help("A path to search recursively for symbol files.")
                .multiple(true)
                .number_of_values(1)
                .index(1),
        ).arg(
            Arg::with_name("ids")
                .value_name("UUID")
                .long("uuid")
                .help("Search for specific UUIDs.")
                .validator(validate_uuid)
                .multiple(true)
                .number_of_values(1),
        ).arg(
            Arg::with_name("require_all")
                .long("require-all")
                .help("Errors if not all UUIDs specified with --uuid could be found."),
        ).arg(
            Arg::with_name("symbol_maps")
                .long("symbol-maps")
                .value_name("PATH")
                .help(
                    "Optional path to BCSymbolMap files which are used to \
                     resolve hidden symbols in the actual dSYM files.  This \
                     requires the dsymutil tool to be available.",
                ),
        ).arg(
            Arg::with_name("derived_data")
                .long("derived-data")
                .help("Search for debug symbols in derived data."),
        ).arg(
            Arg::with_name("no_zips")
                .long("no-zips")
                .help("Do not search in ZIP files."),
        ).arg(
            Arg::with_name("info_plist")
                .long("info-plist")
                .value_name("PATH")
                .help(
                    "Optional path to the Info.plist.{n}We will try to find this \
                     automatically if run from Xcode.  Providing this information \
                     will associate the debug symbols with a specific ITC application \
                     and build in Sentry.  Note that if you provide the plist \
                     explicitly it must already be processed.",
                ),
        ).arg(
            Arg::with_name("no_reprocessing")
                .long("no-reprocessing")
                .help("Do not trigger reprocessing after uploading."),
        ).arg(
            Arg::with_name("force_foreground")
                .long("force-foreground")
                .help(
                    "Wait for the process to finish.{n}\
                     By default, the upload process will detach and continue in the \
                     background when triggered from Xcode.  When an error happens, \
                     a dialog is shown.  If this parameter is passed Xcode will wait \
                     for the process to finish before the build finishes and output \
                     will be shown in the Xcode build output.",
                ),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<(), Error> {
    upload_dif::execute_legacy(matches)
}
