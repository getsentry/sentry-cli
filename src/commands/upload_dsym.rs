//! Implements a command for uploading dSYM files.
use clap::{App, AppSettings, ArgMatches};
use failure::Error;

use commands::upload_dif;
use utils::args::{validate_org, validate_project, validate_uuid};

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    clap_app!(@app (app)
        (about: "DEPRECATED: Upload Mac debug symbols to a project.")
        (setting: AppSettings::Hidden)
        (@arg org: -o --org [ORGANIZATION] {validate_org} "The organization slug.")
        (@arg project: -p --project [PROJECT] {validate_project} "The project slug.")
        (@arg paths: [PATH]... "A path to search recursively for symbol files.")
        (@arg ids: --uuid [UUID]... {validate_uuid} "Search for specific UUID.")
        (@arg require_all: --("require-all")
            "Errors if not all identifiers specified with --uuid could be found.")
        (@arg symbol_maps: --("symbol-maps") [PATH]
            "Optional path to BCSymbolMap files which are used to \
             resolve hidden symbols in dSYM files downloaded from \
             iTunes Connect.  This requires the dsymutil tool to be \
             available.")
        (@arg derived_data: --("derived-data") "Search for debug symbols in Xcode's derived data.")
        (@arg no_zips: --("no-zips") "Do not search in ZIP files.")
        (@arg info_plist: --("info-plist") [PATH]
            "Optional path to the Info.plist.{n}We will try to find this \
             automatically if run from Xcode.  Providing this information \
             will associate the debug symbols with a specific ITC application \
             and build in Sentry.  Note that if you provide the plist \
             explicitly it must already be processed.")
        (@arg no_reprocessing: --("no-reprocessing") "Do not trigger reprocessing after uploading.")
        (@arg force_foreground: --("force-foreground")
            "Wait for the process to finish.{n}\
             By default, the upload process will detach and continue in the \
             background when triggered from Xcode.  When an error happens, \
             a dialog is shown.  If this parameter is passed Xcode will wait \
             for the process to finish before the build finishes and output \
             will be shown in the Xcode build output.")
    )
}

pub fn execute(matches: &ArgMatches) -> Result<(), Error> {
    upload_dif::execute_legacy(matches)
}
