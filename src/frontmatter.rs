use std::ops::Deref;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// Very basic YAML front matter parser.
#[derive(Debug, Serialize, Deserialize)]
pub struct FrontMatter<T> {
	/// The content past the front matter.
	pub content: String,
	/// The front matter found, if any.
	#[serde(flatten)]
	pub data: Option<T>,
}

impl<T> FrontMatter<T> {
	/// Creates a new front matter.
	pub fn new(data: Option<T>, content: String) -> Self {
		Self { data, content }
	}

	/// Creates a new front matter without content.
	pub fn new_empty(data: Option<T>) -> Self {
		Self::new(data, String::new())
	}
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

/// Wrapper around `FrontMatter` to only function when the data is present.
#[derive(Debug, Serialize, Deserialize)]
pub struct FrontMatterRequired<T>(FrontMatter<T>);

impl<T> FrontMatterRequired<T> {
	/// Creates a new front matter.
	pub fn new(data: T, content: String) -> Self {
		Self(FrontMatter::new(Some(data), content))
	}

	/// Creates a new front matter without content.
	pub fn new_empty(data: T) -> Self {
		Self(FrontMatter::new_empty(Some(data)))
	}

	/// Gets a reference to the front matter's data.
	pub fn data(&self) -> &T {
		self.0.data.as_ref().expect("missing front matter data")
	}

	/// Gets a mutable reference to the front matter's data.
	pub fn data_mut(&mut self) -> &mut T {
		self.0.data.as_mut().expect("missing front matter data")
	}

	/// Gets a mutable reference to the front matter's content.
	pub fn content_mut(&mut self) -> &mut String {
		&mut self.0.content
	}
}

impl<T> FrontMatterRequired<T>
where
	T: DeserializeOwned,
{
	/// Parses the given input for front matter, failing if missing.
	pub fn parse(input: String) -> eyre::Result<Self> {
		let fm = FrontMatter::parse(input)?;
		if fm.data.is_none() {
			eyre::bail!("missing frontmatter!");
		}
		Ok(Self(fm))
	}
}

impl<T> Deref for FrontMatterRequired<T> {
	type Target = FrontMatter<T>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
