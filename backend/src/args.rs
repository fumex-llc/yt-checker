use std::net::SocketAddr;

use clap::{Parser, ValueEnum};

#[derive(Clone, Parser, ValueEnum)]
pub enum Strategy {
    RoundRobin,
    Random,
}

#[derive(Parser)]
pub struct Args {
    #[arg(short, long, default_value = "0.0.0.0:8080")]
    pub listen: SocketAddr,
    #[arg(short, long, default_value = "ytcheck")]
    pub url_handler: String,
    #[arg(short, long)]
    pub urls: Vec<String>,
    #[arg(short, long, default_value="720")]
    pub min_h: u16,
    #[arg(short, long, default_value="10")]
    pub timeout: u16,
    #[arg(short, long, value_enum, default_value_t=Strategy::Random)]
    pub strategy: Strategy,
}