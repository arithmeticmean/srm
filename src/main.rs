mod cli;
mod core;
mod error;
mod trash;

use crate::cli::*;
use crate::error::{ExitCode, RMSError};
fn main() {
    srm_run();
}

static VERSION: &str = env!("CARGO_PKG_VERSION");

static HELP_SHORT: &str = "\
                          Try \"srm --help\" for more information
                          ";

static HELP_LONG: &str = "\
  Usage: srm [OPTION]... [FILE]...
Move FILE(s) to Trash.

  -f, --force           ignore nonexistent files and arguments, never prompt
  -r, -R, --recursive   remove directories and their contents recursively
  -d, --dir             remove empty directories
  -v, --verbose         explain what is being done
      --help        display this help and exit
      --version     output version information and exit
";

pub fn srm_run() {
    let mut exitcode = ExitCode::Success;

    match SRMArgs::build() {
        Ok(args) => args.handle_args(&mut exitcode),
        Err(err) => match err {
            RMSError::Help => println!("{HELP_LONG}"),
            RMSError::Version => println!("{VERSION}"),
            _ => {
                eprintln!("{err}\n{HELP_SHORT}");
                exitcode.update(ExitCode::UsageError);
            }
        },
    }
    std::process::exit(exitcode.code());
}
