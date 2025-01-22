use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
  #[arg(short, long)]
  pub game_dir: Option<PathBuf>,
  #[command(subcommand)]
  pub subcommand: SubCommand,
}

#[derive(Subcommand)]
#[command(about, long_about = None)]
pub enum SubCommand {
  Install {
    #[arg()]
    items: Vec<String>,
  },
  Uninstall {
    #[arg()]
    items: Vec<String>,
  },
  Update {},
}
