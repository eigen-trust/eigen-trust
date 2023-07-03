//! # Filesystem Actions Module.
//!
//! This module provides functionalities for filesystem actions.

use serde::de::DeserializeOwned;
use serde_json::from_reader;
use std::{
	env::current_dir,
	fs::{read, read_to_string, write, File},
	io::{BufReader, Result},
	path::PathBuf,
};

/// Enum representing the possible file extensions.
pub enum FileType {
	/// Binary file.
	Bin,
	/// CSV file.
	Csv,
	/// JSON file.
	Json,
	/// Rust file.
	Rs,
	/// Yul file.
	Yul,
}

impl FileType {
	/// Converts the enum variant into its corresponding file extension.
	fn as_str(&self) -> &'static str {
		match self {
			FileType::Bin => "bin",
			FileType::Csv => "csv",
			FileType::Json => "json",
			FileType::Rs => "rs",
			FileType::Yul => "yul",
		}
	}
}

/// Retrieves the path to the `data` directory.
pub fn get_data_directory() -> Result<PathBuf> {
	let current_dir = current_dir()?;

	// Workaround for the tests running in the `client` directory.
	#[cfg(test)]
	{
		Ok(current_dir.join("../data"))
	}

	#[cfg(not(test))]
	{
		Ok(current_dir.join("data"))
	}
}

/// Helper function to get the path of a file in the `data` directory.
pub fn get_file_path(file_name: &str, file_type: FileType) -> Result<PathBuf> {
	let current_dir = get_data_directory()?;
	Ok(current_dir.join(format!("{}.{}", file_name, file_type.as_str())))
}

/// Reads a binary file from the `data` directory and returns its contents as bytes.
pub fn read_binary(file_name: &str) -> Result<Vec<u8>> {
	let bin_path = get_file_path(file_name, FileType::Bin)?;
	read(bin_path)
}

/// Writes bytes to a binary file in the `data` directory.
pub fn write_binary(bytes: Vec<u8>, file_name: &str) -> Result<()> {
	let bin_path = get_file_path(file_name, FileType::Bin)?;
	write(bin_path, bytes)
}

/// Reads a JSON file from the `data` directory and returns its deserialized contents.
pub fn read_json<T: DeserializeOwned>(file_name: &str) -> Result<T> {
	let json_path = get_file_path(file_name, FileType::Json)?;
	let file = File::open(json_path)?;
	let reader = BufReader::new(file);
	from_reader(reader).map_err(Into::into)
}

/// Reads a file from the `data` directory and returns its contents as a string.
pub fn read_yul(file_name: &str) -> Result<String> {
	let yul_path = get_file_path(file_name, FileType::Yul)?;
	read_to_string(yul_path)
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde::Deserialize;
	use std::fs::{self, File};
	use std::io::Write;

	#[derive(Deserialize, Debug, PartialEq)]
	struct TestStruct {
		field: String,
	}

	#[test]
	fn test_read_binary() {
		let file_name = "test_read_binary";

		// Write test file
		let mut file = File::create(get_file_path(file_name, FileType::Bin).unwrap()).unwrap();
		file.write_all(b"binary data").unwrap();

		// Test reading
		let data = read_binary(file_name).unwrap();
		assert_eq!(data, b"binary data");

		// Cleanup
		fs::remove_file(get_file_path(file_name, FileType::Bin).unwrap()).unwrap();
	}

	#[test]
	fn test_write_binary() {
		let file_name = "test_write_binary";
		let binary_data: Vec<u8> = vec![0xff, 0x61, 0x4a, 0x6d, 0x59, 0x56, 0x2a, 0x42, 0x37, 0x72];

		// Write binary data
		write_binary(binary_data.clone(), file_name).unwrap();

		// Test if the file was written correctly
		let data = fs::read(get_file_path(file_name, FileType::Bin).unwrap()).unwrap();
		assert_eq!(data, binary_data);

		// Cleanup
		fs::remove_file(get_file_path(file_name, FileType::Bin).unwrap()).unwrap();
	}

	#[test]
	fn test_read_json() {
		let file_name = "test_read_json";

		// Write test file
		let mut file = File::create(get_file_path(file_name, FileType::Json).unwrap()).unwrap();
		file.write_all(b"{ \"field\": \"json data\" }").unwrap();

		// Test reading
		let data: TestStruct = read_json(file_name).unwrap();
		assert_eq!(data, TestStruct { field: "json data".to_string() });

		// Cleanup
		fs::remove_file(get_file_path(file_name, FileType::Json).unwrap()).unwrap();
	}

	#[test]
	fn test_read_yul() {
		let file_name = "test_read_yul";

		// Write test file
		let mut file = File::create(get_file_path(file_name, FileType::Yul).unwrap()).unwrap();
		file.write_all(b"yul data").unwrap();

		// Test reading
		let data = read_yul(file_name).unwrap();
		assert_eq!(data, "yul data");

		// Cleanup
		fs::remove_file(get_file_path(file_name, FileType::Yul).unwrap()).unwrap();
	}
}
