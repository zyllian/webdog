//! Module containing the site builder.

use std::{collections::HashMap, path::PathBuf};

use eyre::{Context, OptionExt, eyre};
use lol_html::{HtmlRewriter, Settings, element, html_content::ContentType};
use rayon::prelude::*;
use serde::Serialize;
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};
use tera::Tera;
use url::Url;

use crate::{PageMetadata, ROOT_PATH, SASS_PATH, Site, resource::ResourceBuilder, util};

/// Default path for static webdog resources included with the site build.
const WEBDOG_DEFAULT_PATH: &str = "webdog";

/// Struct containing data to be sent to templates when rendering them.
#[derive(Debug, Serialize)]
struct TemplateData<'a, T> {
	/// The rendered page.
	pub page: &'a str,
	/// The page's title.
	pub title: &'a str,
	/// Custom template data.
	pub data: T,
	/// Userdata supplied from the page.
	pub userdata: serde_yaml_ng::Value,
}

/// Struct used to build the site.
pub struct SiteBuilder {
	/// The Tera registry used to render templates.
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
	pub serving: bool,

	/// The resource builders available to the builder.
	pub resource_builders: HashMap<String, ResourceBuilder>,
}

impl SiteBuilder {
	/// Creates a new site builder.
	pub fn new(site: Site, serving: bool) -> eyre::Result<Self> {
		let mut build_path = match &site.config.build {
			Some(build) => site.site_path.join(build),
			_ => site.site_path.join("build"),
		};
		if serving {
			build_path = site.site_path.join("build");
		}

		let mut tera = Tera::new(
			site.site_path
				.join(format!("{}/**/*.tera", crate::TEMPLATES_PATH))
				.to_str()
				.expect("failed to convert path to string"),
		)?;
		tera.autoescape_on(vec![".tera"]);

		Ok(Self {
			tera,
			syntax_set: SyntaxSet::load_defaults_newlines(),
			theme_set: ThemeSet::load_defaults(),
			resource_builders: HashMap::new(),
			site,
			build_path,
			serving,
		})
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

		let webdog_path = self.build_path.join(
			self.site
				.config
				.webdog_path
				.clone()
				.unwrap_or_else(|| WEBDOG_DEFAULT_PATH.to_string()),
		);
		std::fs::create_dir(&webdog_path)?;
		std::fs::write(
			webdog_path.join("webdog.js"),
			include_str!("./embedded/js/webdog.js"),
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
	#[allow(clippy::too_many_arguments)]
	pub fn rewrite_html(
		&self,
		html: String,
		title: &str,
		head: &Option<String>,
		scripts: &[String],
		styles: &[String],
		is_partial: bool,
		webdog_path: &str,
	) -> eyre::Result<String> {
		use kuchikiki::traits::*;

		let html = {
			let document = kuchikiki::parse_html().one(html.clone()).document_node;
			let mut needs_reserialized = false;

			while let Ok(el) = document.select_first("wd-partial") {
				needs_reserialized = true;
				let attr_map = el.attributes.borrow();
				let template = attr_map
					.get("t")
					.ok_or_eyre("missing t attribute on wd-partial")?;
				let attr_map: HashMap<_, _> = attr_map
					.map
					.iter()
					.map(|(k, v)| (k.local.to_string(), &v.value))
					.collect();
				let mut html_buf = Vec::new();
				for child in el.as_node().children() {
					child.serialize(&mut html_buf)?;
				}
				let html = String::from_utf8(html_buf)?;
				let new_html = self.build_page_raw(
					PageMetadata {
						template: Some(template.to_string()),
						userdata: serde_yaml_ng::to_value(attr_map)?,
						is_partial: true,
						..Default::default()
					},
					&html,
					(),
				)?;
				let new_doc = kuchikiki::parse_html()
					.one(new_html)
					.document_node
					.select_first("body")
					.map(|b| b.as_node().children())
					.expect("should never fail");
				for child in new_doc {
					el.as_node().insert_before(child);
				}
				el.as_node().detach();
			}

			if needs_reserialized {
				let mut html = Vec::new();
				document.serialize(&mut html)?;
				String::from_utf8(html)?
			} else {
				html
			}
		};

		let output = if is_partial {
			html
		} else {
			let mut output = Vec::new();
			let mut rewriter = HtmlRewriter::new(
				Settings {
					element_content_handlers: vec![
						element!("body", |el| {
							if self.serving {
								el.set_attribute("class", "debug")?;
							}
							Ok(())
						}),
						element!("head", |el| {
							el.prepend(r#"<meta charset="utf-8">"#, ContentType::Html);
							el.append(&format!("<title>{title}</title>"), ContentType::Html);
							if let Some(head) = head {
								el.append(head, ContentType::Html);
							}
							for script in scripts {
								el.append(
									&format!(
										r#"<script type="text/javascript" src="{script}" defer></script>"#
									),
									ContentType::Html,
								);
							}
							for style in styles {
								el.append(
									&format!(r#"<link rel="stylesheet" href="/styles/{style}">"#),
									ContentType::Html,
								);
							}
							el.append(
								&format!(
									r#"<script type="text/javascript" src="/{webdog_path}/webdog.js" defer></script>"#
								),
								ContentType::Html,
							);
							if self.serving {
								el.append(r#"<script src="/_dev.js"></script>"#, ContentType::Html);
							}

							Ok(())
						}),
						element!("img", |el| {
							if let Some(mut src) = el.get_attribute("src")
								&& let Some((command, new_src)) = src.split_once('$')
							{
								let mut new_src = new_src.to_string();
								#[allow(clippy::single_match)]
								match command {
									"cdn" => {
										new_src = self.site.config.cdn_url(&new_src)?.to_string();
									}
									_ => new_src = src,
								}
								src = new_src;
								el.set_attribute("src", &src)?;
							}

							Ok(())
						}),
						element!("a", |el| {
							if let Some(mut href) = el.get_attribute("href") {
								if let Some((command, new_href)) = href.split_once('$') {
									let mut new_href = new_href.to_string();
									match command {
										"me" => {
											el.set_attribute(
												"rel",
												&(el.get_attribute("rel").unwrap_or_default()
													+ " me"),
											)?;
										}
										"cdn" => {
											new_href =
												self.site.config.cdn_url(&new_href)?.to_string();
										}
										_ => {
											new_href = href;
										}
									}
									href = new_href;
									el.set_attribute("href", &href)?;
								}
								if let Ok(url) = Url::parse(&href)
									&& url.host().is_some()
								{
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

			String::from_utf8(output)?
		};

		Ok(output)
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

		let head = if let Some(embed) = page_metadata.embed {
			Some(embed.build(self)?)
		} else {
			None
		};

		let out = self.tera.render(
			&page_metadata
				.template
				.unwrap_or_else(|| "base.tera".to_string()),
			&tera::Context::from_serialize(TemplateData {
				page: page_html,
				title: &title,
				data: extra_data,
				userdata: page_metadata.userdata,
			})?,
		)?;

		// Modify HTML output
		let mut out = self.rewrite_html(
			out,
			&title,
			&head,
			&page_metadata.scripts,
			&page_metadata.styles,
			page_metadata.is_partial,
			&self
				.site
				.config
				.webdog_path
				.clone()
				.unwrap_or_else(|| WEBDOG_DEFAULT_PATH.to_string()),
		)?;

		if let Some(data) = extra
			&& let Some(extra) = crate::extras::get_extra(&data.name)
		{
			out = extra.handle(out, self, &data)?;
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

		let page_html = util::render_markdown(self, &page.content)?;

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

	/// Builds all of the site's standard pages.
	pub fn build_all_pages(&self) -> eyre::Result<()> {
		self.site
			.page_index
			.keys()
			.par_bridge()
			.try_for_each(|page_name| self.build_page(page_name))?;
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

	/// Builds the entire site.
	pub fn build_all(&self) -> eyre::Result<()> {
		self.build_all_pages()?;
		self.build_sass()?;

		for (_source_path, config) in self.site.config.resources.iter() {
			let mut res_builder = ResourceBuilder::new(config.clone());
			res_builder.load_all(self)?;
			res_builder.build_all(self)?;
		}

		Ok(())
	}
}
