//! Module containing various utilities.

use std::path::Path;

use pulldown_cmark::{Options, Parser};
use time::OffsetDateTime;

use crate::builder::SiteBuilder;

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

/// Helper to render markdown.
pub fn render_markdown(builder: &SiteBuilder, input: &str) -> eyre::Result<String> {
	let mut language = None;
	let parser = Parser::new_ext(input, Options::all()).filter_map(|event| {
		// syntax highlighting for code blocks
		match event {
			pulldown_cmark::Event::Start(pulldown_cmark::Tag::CodeBlock(
				pulldown_cmark::CodeBlockKind::Fenced(name),
			)) => {
				language = Some(name);
				None
			}
			pulldown_cmark::Event::Text(code) => {
				if let Some(language) = language.take() {
					let syntax_reference = builder
						.syntax_set
						.find_syntax_by_token(&language)
						.unwrap_or_else(|| builder.syntax_set.find_syntax_plain_text());
					let html = format!(
						r#"<div class="wd-codeblock">
							<button class="copy">Copy</button>
							{}
						</div>"#,
						syntect::html::highlighted_html_for_string(
							&code,
							&builder.syntax_set,
							syntax_reference,
							builder.theme_set
								.themes
								.get(&builder.site.config.code_theme)
								.as_ref()
								.expect("should never fail"),
						)
						.expect("failed to highlight syntax")
					);
					Some(pulldown_cmark::Event::Html(html.into()))
				} else {
					Some(pulldown_cmark::Event::Text(code))
				}
			}
			_ => Some(event),
		}
	});
	let mut page_html = String::new();
	pulldown_cmark::html::push_html(&mut page_html, parser);

	Ok(page_html)
}
