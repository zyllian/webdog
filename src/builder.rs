//! Module containing the site builder.

use std::{collections::HashMap, path::PathBuf};

use eyre::{eyre, Context, OptionExt};
use lol_html::{element, html_content::ContentType, HtmlRewriter, Settings};
use pulldown_cmark::{Options, Parser};
use serde::Serialize;
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};
use tera::Tera;
use url::Url;

use crate::{resource::ResourceBuilder, util, PageMetadata, Site, ROOT_PATH, SASS_PATH};

/// Path for static webdog resources included with the site build.
const WEBDOG_PATH: &str = "webdog";

/// Struct containing data to be sent to templates when rendering them.
#[derive(Debug, Serialize)]
struct TemplateData<'a, T> {
	/// The rendered page.
	pub page: &'a str,
	/// The page's title.
	pub title: &'a str,
	/// Custom head data for the page.
	pub head: Option<String>,
	/// The page's custom scripts.
	pub scripts: &'a [String],
	/// the page's custom styles.
	pub styles: &'a [String],
	/// Custom template data.
	#[serde(flatten)]
	pub extra_data: T,
}

/// Struct used to build the site.
pub struct SiteBuilder {
	/// The Handlebars registry used to render templates.
	pub(crate) tera: Tera,
	/// The syntax set used to render source code.
	pub(crate) syntax_set: SyntaxSet,
	/// The theme set used to render source code.
	pub(crate) theme_set: ThemeSet,
	/// The site info used to build the site.
	pub site: Site,
	/// The path to the build directory.
	pub build_path: PathBuf,
	/// Whether the site is going to be served locally with the dev server.
	serving: bool,

	/// The resource builders available to the builder.
	pub resource_builders: HashMap<String, ResourceBuilder>,
}

impl SiteBuilder {
	/// Creates a new site builder.
	pub fn new(site: Site, serving: bool) -> Self {
		let mut build_path = match &site.config.build {
			Some(build) => site.site_path.join(build),
			_ => site.site_path.join("build"),
		};
		if serving {
			build_path = site.site_path.join("build");
		}

		let mut tera = Tera::new(
			site.site_path
				.join("templates/**/*.tera")
				.to_str()
				.expect("failed to convert path to string"),
		)
		.expect("failed to create tera instance");
		tera.autoescape_on(vec![".tera"]);

		Self {
			tera,
			syntax_set: SyntaxSet::load_defaults_newlines(),
			theme_set: ThemeSet::load_defaults(),
			resource_builders: HashMap::new(),
			site,
			build_path,
			serving,
		}
	}

	/// Prepares the site builder for use and sets up the build directory.
	pub fn prepare(mut self) -> eyre::Result<Self> {
		self.tera.full_reload()?;
		if self.build_path.exists() {
			for entry in self.build_path.read_dir()? {
				let path = &entry?.path();
				if path.is_dir() {
					std::fs::remove_dir_all(path).with_context(|| {
						format!("Failed to remove directory at {}", path.display())
					})?;
				} else {
					std::fs::remove_file(path)
						.with_context(|| format!("Failed to remove file at {}", path.display()))?;
				}
			}
		} else {
			std::fs::create_dir(&self.build_path).wrap_err("Failed to create build directory")?;
		}

		let webdog_path = self.build_path.join(WEBDOG_PATH);
		std::fs::create_dir(&webdog_path)?;
		std::fs::write(
			webdog_path.join("webdog.js"),
			include_str!("./js/webdog.js"),
		)?;

		let root_path = self.site.site_path.join(ROOT_PATH);
		if root_path.exists() {
			for entry in walkdir::WalkDir::new(&root_path) {
				let entry = entry?;
				let path = entry.path();
				if path.is_dir() {
					continue;
				}
				let output_path = self.build_path.join(path.strip_prefix(&root_path)?);
				let parent_path = output_path.parent().expect("should never fail");
				std::fs::create_dir_all(parent_path)?;
				std::fs::copy(path, output_path)?;
			}
		}

		self.reload()?;

		Ok(self)
	}

	/// Performs actions that need to be done when the config changes while serving.
	pub fn reload(&mut self) -> eyre::Result<()> {
		self.site
			.config
			.check(self)
			.wrap_err("site config failed check:")?;
		self.resource_builders.clear();
		for (prefix, config) in &self.site.config.resources {
			self.resource_builders
				.insert(prefix.to_owned(), ResourceBuilder::new(config.clone()));
		}

		for prefix in self.resource_builders.keys().cloned().collect::<Vec<_>>() {
			self.reload_resource_builder(&prefix)?;
		}

		Ok(())
	}

	/// Reloads a particular resource builder's metadata.
	pub fn reload_resource_builder(&mut self, builder: &str) -> eyre::Result<()> {
		let mut resource_builder = self
			.resource_builders
			.remove(builder)
			.ok_or_else(|| eyre!("missing resource builder: {builder}"))?;
		resource_builder.load_all(self)?;
		self.resource_builders
			.insert(builder.to_string(), resource_builder);
		Ok(())
	}

	/// Function to rewrite HTML wow.
	pub fn rewrite_html(&self, html: String) -> eyre::Result<String> {
		let mut output = Vec::new();
		let mut rewriter = HtmlRewriter::new(
			Settings {
				element_content_handlers: vec![
					#[cfg(debug_assertions)]
					element!("body", |el| {
						el.set_attribute("class", "debug")?;
						Ok(())
					}),
					element!("head", |el| {
						el.prepend(r#"<meta charset="utf-8">"#, ContentType::Html);
						el.append(
							r#"<script type="text/javascript" src="/webdog/webdog.js" defer></script>"#,
							ContentType::Html,
						);
						if self.serving {
							el.append(r#"<script src="/_dev.js"></script>"#, ContentType::Html);
						}

						Ok(())
					}),
					element!("a", |el| {
						if let Some(mut href) = el.get_attribute("href") {
							if let Some((command, mut new_href)) = href.split_once('$') {
								#[allow(clippy::single_match)]
								match command {
									"me" => {
										el.set_attribute(
											"rel",
											&(el.get_attribute("rel").unwrap_or_default() + " me"),
										)?;
									}
									_ => {
										new_href = &href;
									}
								}
								href = new_href.to_string();
								el.set_attribute("href", &href)?;
							}
							if let Ok(url) = Url::parse(&href) {
								if url.host().is_some() {
									// Make external links open in new tabs without referral information
									el.set_attribute(
										"rel",
										(el.get_attribute("rel").unwrap_or_default()
											+ " noopener noreferrer")
											.trim(),
									)?;
									el.set_attribute("target", "_blank")?;
								}
							}
						}

						Ok(())
					}),
					element!("md", |el| {
						el.remove();
						let class = el.get_attribute("class");

						let md_type = el
							.get_attribute("type")
							.ok_or_eyre("missing type attribute on markdown tag")?;

						if md_type == "blog-image" {
							let mut src = el
								.get_attribute("src")
								.ok_or_eyre("missing src attribute")?;

							if src.starts_with("cdn$") {
								src = self.site.config.cdn_url(&src[4..])?.to_string();
							}

							let class = format!("image {}", class.unwrap_or_default());
							let content = el
								.get_attribute("content")
								.ok_or_eyre("missing content attribute")?;

							el.replace(
								&format!(
									r#"
									<div class="{class}">
										<a href="{src}">
											<img src="{src}">
										</a>
										<span>{content}</span>
									</div>
									"#
								),
								ContentType::Html,
							);
						} else {
							return Err(eyre!("unknown markdown tag type: {md_type}").into());
						}

						Ok(())
					}),
				],
				strict: true,
				..Default::default()
			},
			|c: &[u8]| output.extend_from_slice(c),
		);

		rewriter.write(html.as_bytes())?;
		rewriter.end()?;

		Ok(String::from_utf8(output)?)
	}

	/// Helper to build a page without writing it to disk.
	pub fn build_page_raw<T>(
		&self,
		mut page_metadata: PageMetadata,
		page_html: &str,
		extra_data: T,
	) -> eyre::Result<String>
	where
		T: Serialize,
	{
		let extra = page_metadata.extra.take();

		let title = match &page_metadata.title {
			Some(page_title) => format!("{} / {}", self.site.config.title, page_title),
			_ => self.site.config.title.clone(),
		};

		let head = page_metadata.embed.map(|mut embed| {
			embed.site_name.clone_from(&self.site.config.title);
			embed.build()
		});

		let out = self.tera.render(
			&page_metadata
				.template
				.unwrap_or_else(|| "base.tera".to_string()),
			&tera::Context::from_serialize(TemplateData {
				page: page_html,
				title: &title,
				head,
				scripts: &page_metadata.scripts,
				styles: &page_metadata.styles,
				extra_data,
			})?,
		)?;

		// Modify HTML output
		let mut out = self.rewrite_html(out)?;

		if let Some(data) = extra {
			if let Some(extra) = crate::extras::get_extra(&data.name) {
				out = extra.handle(out, self, &data)?;
			}
		}

		if !self.serving {
			out = minifier::html::minify(&out);
		}

		Ok(out)
	}

	/// Builds a standard page.
	pub fn build_page(&self, page_name: &str) -> eyre::Result<()> {
		let page_path = self.site.page_index.get(page_name).expect("Missing page");

		let input = std::fs::read_to_string(page_path)
			.with_context(|| format!("Failed to read page at {}", page_path.display()))?;
		let page = crate::frontmatter::FrontMatter::parse(input)?;

		let mut language = None;
		let parser = Parser::new_ext(&page.content, Options::all()).filter_map(|event| {
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
						let syntax_reference = self
							.syntax_set
							.find_syntax_by_token(&language)
							.unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
						let html = format!(
							r#"<div class="wd-codeblock">
								<button class="copy">Copy</button>
								{}
							</div>"#,
							syntect::html::highlighted_html_for_string(
								&code,
								&self.syntax_set,
								syntax_reference,
								self.theme_set
									.themes
									.get(&self.site.config.code_theme)
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

		let out = self.build_page_raw(page.data.unwrap_or_default(), &page_html, ())?;

		let out_path = self.build_path.join(page_name).with_extension("html");
		std::fs::create_dir_all(out_path.parent().unwrap())
			.with_context(|| format!("Failed to create directory for page {}", page_name))?;
		std::fs::write(&out_path, out).with_context(|| {
			format!(
				"Failed to create HTML file at {} for page {}",
				out_path.display(),
				page_name
			)
		})?;

		Ok(())
	}

	/// Builds the Sass styles in the site.
	pub fn build_sass(&self) -> eyre::Result<()> {
		let styles_path = self.build_path.join("styles");
		if !styles_path.exists() {
			std::fs::create_dir(&styles_path)?;
		}
		if self.serving {
			util::remove_dir_contents(&styles_path)
				.wrap_err("Failed to remove old contents of styles directory")?;
		}
		let sass_path = self.site.site_path.join(SASS_PATH);
		for sheet in &self.site.config.sass_styles {
			let sheet_path = sass_path.join(sheet);
			if let Some(sheet_path) = sheet_path.to_str() {
				match grass::from_path(sheet_path, &grass::Options::default()) {
					Ok(mut css) => {
						if !self.serving {
							css = minifier::css::minify(&css)
								.map_err(|err| eyre::anyhow!(err))?
								.to_string();
						}
						std::fs::write(styles_path.join(sheet).with_extension("css"), css)
							.with_context(|| {
								format!("Failed to write new CSS file for Sass: {:?}", sheet)
							})?;
					}
					Err(e) => eprintln!(
						"Failed to compile Sass stylesheet at {:?}: {}",
						sheet_path, e
					),
				}
			} else {
				eprintln!(
					"Sass stylesheet path contains invalid UTF-8: {:?}",
					sheet_path
				);
			}
		}

		Ok(())
	}

	/// Builds all resource types.
	pub fn build_all_resources(&self) -> eyre::Result<()> {
		for builder in self.resource_builders.values() {
			builder.build_all(self)?;
		}
		Ok(())
	}

	/// Builds a resource type from the site.
	pub fn build_resources(&self, resource: &str) -> eyre::Result<()> {
		self.resource_builders
			.get(resource)
			.ok_or_else(|| eyre!("missing resource: {resource}"))?
			.build_all(self)
	}
}
