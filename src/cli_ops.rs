use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "public_trading", about = "Public Trading CLI")]
pub struct Cli {
    /// Print the account portfolio and exit
    #[arg(long)]
    pub show_portfolio: bool,
}
