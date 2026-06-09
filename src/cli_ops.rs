use clap::{Parser, Subcommand};
use tracing::Level;

#[derive(Parser, Debug)]
#[command(name = "public_trading", about = "Public Trading CLI")]
pub struct Cli {
    /// Log level (error, warn, info, debug, trace)
    #[arg(long, short = 'L', default_value = "info", global = true)]
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

    /// Analyze single Option to choose good entries
    AnalyzeOption {
        /// Symbol of underlying to analyze options for
        symbol: String,

        /// Expiration to analyze, like "2026-02-27"
        expiration: String,
    },

    /// Analyze Options from multiple Equities to choose good entries
    AnalyzeOptions {
        /// Expiration to analyze, like "2026-02-27"
        expiration: String,
        /// Group of equities to analyze, read from config file.
        #[arg(short = 'g')]
        equities_group: String,
    },

    /// Monitor open options positions and suggest or execute exits
    OptionsStopper {
        /// Gain-percent threshold below which a position should be exited (e.g. -200.0)
        #[arg(long, default_value = "-200.0")]
        threshold: f64,

        /// Print actions without fetching live quotes or placing orders
        #[arg(long)]
        dry_run: bool,

        /// Print actions of attempting an exit, but don't execute it.
        #[arg(long)]
        dry_run_exit: bool,
    },
}
