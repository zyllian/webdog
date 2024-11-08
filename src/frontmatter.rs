use serde::{de::DeserializeOwned, Serialize};

/// Very basic YAML front matter parser.
#[derive(Debug)]
pub struct FrontMatter<T> {
	/// The content past the front matter.
	pub content: String,
	/// The front matter found, if any.
	pub data: Option<T>,
}

impl<T> FrontMatter<T>
where
	T: DeserializeOwned,
{
	/// Parses the given input for front matter.
	pub fn parse(input: String) -> eyre::Result<Self> {
		if input.starts_with("---\n") {
			if let Some((frontmatter, content)) = input[3..].split_once("\n---\n") {
				let data = serde_yml::from_str(frontmatter)?;
				return Ok(Self {
					content: content.to_string(),
					data,
				});
			}
		}
		Ok(Self {
			content: input,
			data: None,
		})
	}
}

impl<T> FrontMatter<T>
where
	T: Serialize,
{
	/// Formats the front matter and content to a string ready for saving.
	pub fn format(&self) -> eyre::Result<String> {
		let mut output = String::new();

		if let Some(data) = &self.data {
			output.push_str("---\n");
			output.push_str(&serde_yml::to_string(data)?);
			output.push_str("---\n\n");
		}

		output.push_str(&self.content);

		if !output.ends_with('\n') {
			output.push('\n');
		}

		Ok(output)
	}
}
