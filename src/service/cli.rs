//! @module service::cli
//! @description Command-line contract of NitroTrayService.exe.
//!
//! @input  Process arguments (excluding argv[0]).
//! @output A `Command`, or a usage error.
//! @dependencies none
//!
//! run        entry point used by the Service Control Manager
//! console    serve the pipe in the foreground (development; add --simulate for the fake backend)
//! install    register + start the LocalSystem service (requires elevation)
//! uninstall  stop + remove the service (requires elevation)

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Run,
    Console { simulate: bool, pipe: Option<String> },
    Install,
    Uninstall,
    Help,
}

pub const USAGE: &str = "usage: NitroTrayService.exe <run|console [--simulate] [--pipe NAME]|install|uninstall>";

pub fn parse(args: &[String]) -> Result<Command, String> {
    let mut it = args.iter().map(String::as_str);
    match it.next() {
        None | Some("-h" | "--help" | "help") => Ok(Command::Help),
        Some("run") => Ok(Command::Run),
        Some("install") => Ok(Command::Install),
        Some("uninstall") => Ok(Command::Uninstall),
        Some("console") => {
            let (mut simulate, mut pipe) = (false, None);
            while let Some(a) = it.next() {
                match a {
                    "--simulate" => simulate = true,
                    "--pipe" => {
                        let name = it.next().ok_or("--pipe needs a name")?;
                        if !name.starts_with(r"\\.\pipe\") {
                            return Err(format!("pipe name must start with \\\\.\\pipe\\: {name}"));
                        }
                        pipe = Some(name.to_string());
                    }
                    other => return Err(format!("unknown option '{other}'\n{USAGE}")),
                }
            }
            Ok(Command::Console { simulate, pipe })
        }
        Some(other) => Err(format!("unknown command '{other}'\n{USAGE}")),
    }
}
