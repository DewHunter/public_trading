use clap::{Parser, Subcommand};
use tracing::Level;

#[derive(Parser, Debug)]
#[command(name = "public_trading", about = "Public Trading CLI")]
pub struct Cli {
    /// Log level (error, warn, info, debug, trace)
    #[arg(long, default_value = "info", global = true)]
    pub log_level: Level,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Print the account portfolio and exit
    ShowPortfolio {
        /// Output raw JSON instead of formatted text
        #[arg(long)]
        json: bool,
    },

    /// Analyze Options to choose good entries
    AnalyzeOptions {
        /// Symbol of underlying to analyze options for
        symbol: String,
        /// Expiration to analyze, like "2026-02-27"
        expiration: String,
    },

    /// Monitor open options positions and suggest or execute exits
    OptionsStopper {
        /// Gain-percent threshold below which a position should be exited (e.g. -200.0)
        #[arg(long, default_value = "-200.0")]
        threshold: f32,

        /// Print actions without fetching live quotes or placing orders
        #[arg(long)]
        dry_run: bool,
    },
}
