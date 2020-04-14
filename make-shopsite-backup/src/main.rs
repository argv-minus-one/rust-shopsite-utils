use std::{
	borrow::Cow,
	env,
	path::PathBuf,
	process::exit
};
use structopt::StructOpt;

mod config;

const BIN_NAME: &str = env!("CARGO_PKG_NAME");
const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), '/', env!("CARGO_PKG_VERSION"));

fn main() {
	#[derive(StructOpt)]
	#[structopt(rename_all = "kebab-case")]
	struct Opts {
		config_path: PathBuf
	}

	let config_path = Opts::from_args().config_path;
}
