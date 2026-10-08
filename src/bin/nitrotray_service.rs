//! @module nitrotray_service
//! @description NitroTrayService.exe entry point: the privileged helper exposing narrow Acer commands.
//!
//! @dependencies nitrotray::service::{cli, host}
use nitrotray::service::cli::{self, Command};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = match cli::parse(&args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let result = match command {
        Command::Help => {
            println!("{}", cli::USAGE);
            Ok(())
        }
        Command::Run => nitrotray::service::host::run_service(),
        Command::Console { simulate, pipe } => nitrotray::service::host::run_console(simulate, pipe),
        Command::Install => nitrotray::service::host::install(),
        Command::Uninstall => nitrotray::service::host::uninstall(),
    };
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
