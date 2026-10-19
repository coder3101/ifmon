mod cli;
mod commands;
mod config;
mod net;
mod settings;
mod theme;
mod tui;
mod units;

use std::io::IsTerminal;
use std::process::ExitCode;

use clap::{CommandFactory, Parser};
use color_eyre::eyre::Result;

use crate::cli::{Cli, Command};
use crate::settings::Settings;

fn main() -> ExitCode {
    // Keeps rich reports for panics; ordinary errors are printed tersely below.
    let _ = color_eyre::install();
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            let prefix = if std::io::stderr().is_terminal() {
                "\x1b[1;31merror:\x1b[0m"
            } else {
                "error:"
            };
            eprintln!("{prefix} {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse_from(cli::normalize_args(std::env::args_os()));

    match &cli.command {
        Some(Command::Completions { shell }) => {
            clap_complete::generate(*shell, &mut Cli::command(), "ifmon", &mut std::io::stdout());
            Ok(())
        }
        Some(Command::Config { action }) => {
            commands::config::run(*action, cli.common.config.as_deref())
        }
        Some(Command::List(args)) => commands::list::run(&Settings::resolve(&cli)?, args),
        Some(Command::Watch(args)) => commands::watch::run(&Settings::resolve(&cli)?, args),
        None => tui::run(Settings::resolve(&cli)?),
    }
}
