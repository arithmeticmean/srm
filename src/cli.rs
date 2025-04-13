use crate::core::srm;
use crate::error::{ExitCode, RMSError};
use crate::trash::TrashDirs;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
struct SRMCliArgs {
    paths: Vec<PathBuf>,
    isrecursive: bool,
    isdirectory: bool,
    isforce: bool,
    isverbose: bool,
}

impl SRMCliArgs {
    fn build() -> Result<SRMCliArgs, RMSError> {
        let mut argsiter = std::env::args();
        argsiter.next();

        Self::parse_args(argsiter)
    }

    fn parse_args<I>(argsiter: I) -> Result<SRMCliArgs, RMSError>
    where
        I: Iterator<Item = String>,
    {
        let mut args = Self {
            paths: Vec::new(),
            isrecursive: false,
            isdirectory: false,
            isforce: false,
            isverbose: false,
        };

        for arg in argsiter {
            match arg.as_str() {
                "--help" => return Err(RMSError::Help),
                "--version" => return Err(RMSError::Version),
                "--recursive" => args.isrecursive = true,
                "--dir" => args.isdirectory = true,
                "--force" => args.isforce = true,
                "--verbose" => args.isverbose = true,
                _ if arg.starts_with("--") => return Err(RMSError::InvalidOption(arg)),
                _ if arg.starts_with("-") && arg.len() > 1 => {
                    for c in arg.chars().skip(1) {
                        match c {
                            'r' => args.isrecursive = true,
                            'd' => args.isdirectory = true,
                            'f' => args.isforce = true,
                            'v' => args.isverbose = true,
                            _ => return Err(RMSError::InvalidOption(format!("{c}"))),
                        }
                    }
                }
                _ => args.paths.push(PathBuf::from(arg)),
            }
        }

        if args.paths.is_empty() {
            return Err(RMSError::MissingOperand);
        }

        Ok(args)
    }
}

pub struct SRMArgs {
    pub srcpaths: Vec<PathBuf>,
    pub trash: TrashDirs,
    pub isrecursive: bool,
    pub isdirectory: bool,
    pub isforce: bool,
    pub isverbose: bool,
}

impl SRMArgs {
    pub fn build() -> Result<SRMArgs, RMSError> {
        let cli_args = SRMCliArgs::build()?;

        let trashdirs = TrashDirs::ensure_trash_dirs()?;

        Ok(Self {
            srcpaths: cli_args.paths,
            trash: trashdirs,
            isverbose: cli_args.isverbose,
            isdirectory: cli_args.isdirectory,
            isrecursive: cli_args.isrecursive,
            isforce: cli_args.isforce,
        })
    }

    pub fn handle_args(&self, exitcode: &mut ExitCode) {
        for srcpath in self.srcpaths.iter() {
            match srm(
                srcpath,
                &self.trash.files,
                self.isverbose,
                self.isdirectory,
                self.isrecursive,
            ) {
                Ok((ref src_path, ref dst_path)) => {
                    if let Err(e) = self.trash.create_trash_info_file(src_path, dst_path) {
                        exitcode.update(ExitCode::RuntimeError);
                        println!("{e}")
                    }
                }
                Err(e) => {
                    exitcode.update(ExitCode::RuntimeError);
                    println!("{e}");
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::SRMCliArgs;

    #[test]
    fn test_parse_args() {
        let argsiter = "-f -r -d -fr -fd -fdr -v file1 file2 file3"
            .split_whitespace()
            .map(|arg| arg.to_string());

        assert_eq!(
            SRMCliArgs::parse_args(argsiter).unwrap(),
            SRMCliArgs {
                paths: vec!["file1".into(), "file2".into(), "file3".into()],
                isforce: true,
                isrecursive: true,
                isdirectory: true,
                isverbose: true,
            }
        );

        let argsiter = "file1 -f file2 -dr file3"
            .split_whitespace()
            .map(|arg| arg.to_string());

        assert_eq!(
            SRMCliArgs::parse_args(argsiter).unwrap(),
            SRMCliArgs {
                paths: vec!["file1".into(), "file2".into(), "file3".into()],
                isforce: true,
                isrecursive: true,
                isdirectory: true,
                isverbose: false,
            }
        );
    }
}
