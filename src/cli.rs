use super::error::RMSCliError;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
pub struct SRMArgs {
    pub paths: Vec<PathBuf>,
    pub isrecursive: bool,
    pub isdirectory: bool,
    pub isforce: bool,
    pub isverbose: bool,
}

impl SRMArgs {
    pub fn parse_cmd_from_env() -> Result<SRMArgs, RMSCliError> {
        let mut argsiter = std::env::args();
        argsiter.next();

        Self::parse_args(argsiter)
    }

    fn parse_args<I>(argsiter: I) -> Result<SRMArgs, RMSCliError>
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
                "--help" => return Err(RMSCliError::Help),
                "--version" => return Err(RMSCliError::Version),
                "--recursive" => args.isrecursive = true,
                "--dir" => args.isdirectory = true,
                "--force" => args.isforce = true,
                "--verbose" => args.isverbose = true,
                _ if arg.starts_with("--") => return Err(RMSCliError::InvalidOption(arg)),
                _ if arg.starts_with("-") && arg.len() > 1 => {
                    for c in arg.chars().skip(1) {
                        match c {
                            'r' => args.isrecursive = true,
                            'd' => args.isdirectory = true,
                            'f' => args.isforce = true,
                            'v' => args.isverbose = true,
                            _ => return Err(RMSCliError::InvalidOption(format!("{c}"))),
                        }
                    }
                }
                _ => args.paths.push(PathBuf::from(arg)),
            }
        }

        if args.paths.is_empty() {
            return Err(RMSCliError::MissingOperand);
        }

        Ok(args)
    }
}

#[cfg(test)]
mod test {
    use super::SRMArgs;

    #[test]
    fn test_parse_args() {
        let argsiter = "-f -r -d -fr -fd -fdr -v file1 file2 file3"
            .split_whitespace()
            .map(|arg| arg.to_string());

        assert_eq!(
            SRMArgs::parse_args(argsiter).unwrap(),
            SRMArgs {
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
            SRMArgs::parse_args(argsiter).unwrap(),
            SRMArgs {
                paths: vec!["file1".into(), "file2".into(), "file3".into()],
                isforce: true,
                isrecursive: true,
                isdirectory: true,
                isverbose: false,
            }
        );
    }
}
