use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "api", about = "Public.com API CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub operation: Operation,
}

#[derive(Subcommand, Debug)]
pub enum Operation {
    GetOptionGreeks {
        /// Option symbols in OSI format
        /// Max 250 per request
        #[arg(long)]
        symbols: Vec<String>,
    },
}
