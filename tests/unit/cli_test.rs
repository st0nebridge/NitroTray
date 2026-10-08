//! @module cli_test
//! @description NitroTrayService.exe argument contract.
use nitrotray::service::cli::{parse, Command, USAGE};

fn args(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

#[test]
fn parses_every_command() {
    assert_eq!(parse(&args(&[])), Ok(Command::Help));
    assert_eq!(parse(&args(&["--help"])), Ok(Command::Help));
    assert_eq!(parse(&args(&["-h"])), Ok(Command::Help));
    assert_eq!(parse(&args(&["help"])), Ok(Command::Help));
    assert_eq!(parse(&args(&["run"])), Ok(Command::Run));
    assert_eq!(parse(&args(&["install"])), Ok(Command::Install));
    assert_eq!(parse(&args(&["uninstall"])), Ok(Command::Uninstall));
    assert_eq!(parse(&args(&["console"])), Ok(Command::Console { simulate: false, pipe: None }));
    assert_eq!(
        parse(&args(&["console", "--simulate", "--pipe", r"\\.\pipe\x"])),
        Ok(Command::Console { simulate: true, pipe: Some(r"\\.\pipe\x".into()) })
    );
}

#[test]
fn rejects_bad_input() {
    assert!(parse(&args(&["explode"])).unwrap_err().contains("unknown command"));
    assert!(parse(&args(&["console", "--fast"])).unwrap_err().contains("unknown option"));
    assert!(parse(&args(&["console", "--pipe"])).unwrap_err().contains("needs a name"));
    assert!(parse(&args(&["console", "--pipe", "C:\\evil"])).unwrap_err().contains("must start with"));
    assert!(USAGE.contains("install"));
}
