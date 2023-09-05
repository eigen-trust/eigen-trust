//! # Filesystem Actions Module.
//!
//! This module provides functionalities for filesystem actions.

use dotenv::{dotenv, var};
use eigentrust::{
	error::EigenError,
	storage::{JSONFileStorage, Storage},
	ClientConfig,
};
use log::warn;
use std::{env::current_dir, path::PathBuf};

const DEFAULT_MNEMONIC: &str = "test test test test test test test test test test test junk";

/// Enum representing the possible file extensions.
pub enum FileType {
	/// CSV file.
	Csv,
	/// JSON file.
	Json,
}

impl FileType {
	/// Converts the enum variant into its corresponding file extension.
	fn as_str(&self) -> &'static str {
		match self {
			FileType::Csv => "csv",
			FileType::Json => "json",
		}
	}
}

/// Retrieves the path to the `assets` directory.
pub fn get_assets_path() -> Result<PathBuf, EigenError> {
	current_dir().map_err(EigenError::IOError).map(|current_dir| {
		// Workaround for the tests running in the `client` directory.
		#[cfg(test)]
		{
			current_dir.join("assets")
		}

		#[cfg(not(test))]
		{
			current_dir.join("eigentrust-cli/assets")
		}
	})
}

/// Helper function to get the path of a file in the `assets` directory.
pub fn get_file_path(file_name: &str, file_type: FileType) -> Result<PathBuf, EigenError> {
	let assets_path = get_assets_path()?;
	Ok(assets_path.join(format!("{}.{}", file_name, file_type.as_str())))
}

/// Loads the configuration file.
pub fn load_config() -> Result<ClientConfig, EigenError> {
	let filepath = get_file_path("config", FileType::Json)?;
	let json_storage = JSONFileStorage::<ClientConfig>::new(filepath);

	json_storage.load()
}

/// Loads the mnemonic from the environment file.
pub fn load_mnemonic() -> String {
	dotenv().ok();
	var("MNEMONIC").unwrap_or_else(|_| {
		warn!("MNEMONIC environment variable is not set. Using default.");
		DEFAULT_MNEMONIC.to_string()
	})
}
