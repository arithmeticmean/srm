use crate::core::srm;
use crate::error::{SRMError, SRMRuntimeError};
use crate::trash::Trash;
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
    fn build() -> Result<SRMCliArgs, SRMError> {
        let mut argsiter = std::env::args();
        argsiter.next();

        Self::parse_args(argsiter)
    }

    fn parse_args<I>(argsiter: I) -> Result<SRMCliArgs, SRMError>
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
                "--help" => return Err(SRMError::Help),
                "--version" => return Err(SRMError::Version),
                "--recursive" => args.isrecursive = true,
                "--dir" => args.isdirectory = true,
                "--force" => args.isforce = true,
                "--verbose" => args.isverbose = true,
                _ if arg.starts_with("--") => return Err(SRMError::InvalidOption(arg)),
                _ if arg.starts_with("-") && arg.len() > 1 => {
                    for c in arg.chars().skip(1) {
                        match c {
                            'r' => args.isrecursive = true,
                            'd' => args.isdirectory = true,
                            'f' => args.isforce = true,
                            'v' => args.isverbose = true,
                            _ => return Err(SRMError::InvalidOption(format!("{c}"))),
                        }
                    }
                }
                _ => args.paths.push(PathBuf::from(arg)),
            }
        }

        if args.paths.is_empty() {
            return Err(SRMError::MissingOperand);
        }

        Ok(args)
    }
}

pub struct SRMArgs {
    pub srcpaths: Vec<PathBuf>,
    pub trash: Trash,
    pub isrecursive: bool,
    pub isdirectory: bool,
    pub isforce: bool,
    pub isverbose: bool,
}

impl SRMArgs {
    pub fn build() -> Result<SRMArgs, SRMError> {
        let cli_args = SRMCliArgs::build()?;

        let trash = crate::trash::Trash::ensure_trash_dirs()?;

        Ok(Self {
            srcpaths: cli_args.paths,
            trash,
            isverbose: cli_args.isverbose,
            isdirectory: cli_args.isdirectory,
            isrecursive: cli_args.isrecursive,
            isforce: cli_args.isforce,
        })
    }

    pub fn handle_command(&self) -> Result<(), SRMRuntimeError> {
        for srcpath in self.srcpaths.iter() {
            let _ = srm(
                srcpath,
                &self.trash.files,
                self.isverbose,
                self.isdirectory,
                self.isrecursive,
            )
            .map(|(ref src_path, ref dst_path)| {
                self.trash.create_trash_info_file(src_path, dst_path)
            })?;
        }
        Ok(())
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
