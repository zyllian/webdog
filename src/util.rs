//! Module containing various utilities.

use std::path::Path;

use time::OffsetDateTime;

/// Simple helper to remove the contents of a directory without removing the directory itself.
pub fn remove_dir_contents(path: &Path) -> eyre::Result<()> {
	for entry in path.read_dir()? {
		let entry = entry?;
		let path = entry.path();
		if path.is_file() {
			std::fs::remove_file(&path)?;
		} else {
			std::fs::remove_dir_all(&path)?;
		}
	}

	Ok(())
}

/// Helper to format a timestamp according to the given format.
pub fn format_timestamp(ts: OffsetDateTime, format: &str) -> eyre::Result<String> {
	let fmt = time::format_description::parse_borrowed::<2>(format)?;
	Ok(ts.format(&fmt)?)
}
