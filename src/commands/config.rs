use std::path::Path;

use color_eyre::eyre::{OptionExt, Result};

use crate::cli::ConfigAction;
use crate::config::{self, DEFAULT_CONFIG};
use crate::theme::Theme;

pub fn run(action: ConfigAction, explicit: Option<&Path>) -> Result<()> {
    let target = || {
        explicit
            .map(Path::to_path_buf)
            .or_else(config::default_path)
            .ok_or_eyre("could not determine a config directory; pass --config <FILE>")
    };
    match action {
        ConfigAction::Path => {
            let path = target()?;
            let note = if path.is_file() {
                ""
            } else {
                " (not created yet)"
            };
            println!("{}{note}", path.display());
        }
        ConfigAction::Init { force } => {
            let path = target()?;
            config::init(&path, force)?;
            println!("Wrote {}", path.display());
        }
        ConfigAction::Default => print!("{DEFAULT_CONFIG}"),
        ConfigAction::Themes => {
            for name in Theme::names() {
                println!("{name}");
            }
        }
    }
    Ok(())
}
