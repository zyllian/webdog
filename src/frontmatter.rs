use serde::de::DeserializeOwned;

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
