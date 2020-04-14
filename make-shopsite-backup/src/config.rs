use serde::Deserialize;
use std::{
	path::PathBuf
};

#[derive(Deserialize)]
pub struct Config {
	backup: BackupConfig,
	shopsite: ShopsiteConfig
}

#[derive(Deserialize)]
pub struct BackupConfig {
	dir: PathBuf
}

#[derive(Deserialize)]
pub struct ShopsiteConfig {
	config_file: PathBuf,
	bo_curl_options: Vec<String>
}
