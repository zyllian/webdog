//! Module containing various utilities.

use std::path::Path;

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
