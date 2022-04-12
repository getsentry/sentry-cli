use anyhow::Result;
use clap::{ArgMatches, Command};

pub mod appcenter;
pub mod gradle;
#[cfg(target_os = "macos")]
pub mod xcode;

macro_rules! each_subcommand {
    ($mac:ident) => {
        $mac!(gradle);
        $mac!(appcenter);
        #[cfg(target_os = "macos")]
        $mac!(xcode);
    };
}

pub fn make_command(mut command: Command) -> Command {
    macro_rules! add_subcommand {
        ($name:ident) => {{
            command = command.subcommand(crate::commands::react_native::$name::make_command(
                Command::new(stringify!($name).replace('_', "-")),
            ));
        }};
    }

    command = command
        .about("Upload build artifacts for react-native projects.")
        .subcommand_required(true)
        .arg_required_else_help(true);
    each_subcommand!(add_subcommand);
    command
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    macro_rules! execute_subcommand {
        ($name:ident) => {{
            if let Some(sub_matches) =
                matches.subcommand_matches(&stringify!($name).replace('_', "-"))
            {
                return crate::commands::react_native::$name::execute(&sub_matches);
            }
        }};
    }
    each_subcommand!(execute_subcommand);
    unreachable!();
}
