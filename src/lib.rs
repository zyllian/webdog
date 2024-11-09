mod builder;
mod extras;
pub mod frontmatter;
mod link_list;
pub mod resource;
#[cfg(feature = "serve")]
pub mod serving;
mod util;

use std::{
	collections::HashMap,
	path::{Path, PathBuf},
};

use extras::ExtraData;
use eyre::Context;
use rayon::prelude::*;
use resource::{EmbedMetadata, ResourceBuilderConfig};
use serde::{Deserialize, Serialize};
use url::Url;
use walkdir::WalkDir;

use builder::SiteBuilder;

/// Source base path for normal site pages.
pub const PAGES_PATH: &str = "pages";
/// Source base path for site templates.
pub const TEMPLATES_PATH: &str = "templates";
/// Source base path for SASS stylesheets.
pub const SASS_PATH: &str = "sass";
/// Source base path for files which will be copied to the site root.
pub const ROOT_PATH: &str = "root";
/// Source base path for resources.
pub const RESOURCES_PATH: &str = "resources";

/// Struct for the site's configuration.
#[derive(Debug, Serialize, Deserialize)]
pub struct SiteConfig {
	/// The location the site is at.
	pub base_url: Url,
	/// The site's title.
	pub title: String,
	/// The site's description? Not sure if this will actually be used or not
	pub description: String,
	/// The site's build directory. Defaults to <site>/build if not specified.
	pub build: Option<String>,
	/// A list of Sass stylesheets that will be built.
	pub sass_styles: Vec<PathBuf>,
	/// URL to the CDN used for the site's images.
	pub cdn_url: Url,
	/// The theme to use for the site's code blocks.
	/// TODO: dark/light themes
	/// TODO: export themes as CSS instead of styling HTML directly
	/// TODO: allow loading user themes
	pub code_theme: String,

	/// List of resources the site should build.
	pub resources: HashMap<String, ResourceBuilderConfig>,
}

impl SiteConfig {
	/// The filename for site config files.
	pub const FILENAME: &str = "config.yaml";

	/// Creates a new site config from the given title.
	pub fn new(base_url: Url, cdn_url: Url, title: String) -> Self {
		Self {
			base_url,
			title,
			description: Default::default(),
			build: None,
			sass_styles: vec!["index.scss".into()],
			cdn_url,
			code_theme: "base16-ocean.dark".to_string(),
			resources: Default::default(),
		}
	}

	/// Gets a CDN url from the given file name.
	pub fn cdn_url(&self, file: &str) -> eyre::Result<Url> {
		Ok(self.cdn_url.join(file)?)
	}

	/// Checks the site config for errors.
	pub fn check(&self, builder: &SiteBuilder) -> eyre::Result<()> {
		builder
			.theme_set
			.themes
			.contains_key(&self.code_theme)
			.then_some(())
			.ok_or_else(|| eyre::eyre!("missing code theme: {}", self.code_theme))?;
		Ok(())
	}

	/// Helper to read the site config from the given path.
	pub fn read(site_path: &Path) -> eyre::Result<Self> {
		let config_path = site_path.join(SiteConfig::FILENAME);
		if !config_path.exists() {
			eyre::bail!("no site config found!");
		}
		Ok(serde_yml::from_str(&std::fs::read_to_string(config_path)?)?)
	}
}

/// Struct for the front matter in templates. (nothing here yet)
#[derive(Debug, Default, Deserialize)]
pub struct TemplateMetadata {}

/// Struct for the front matter in pages.
#[derive(Debug, Default, Deserialize)]
pub struct PageMetadata {
	/// The page's title.
	pub title: Option<String>,
	/// The template to use for the page. If not specified, it defaults to "base".
	pub template: Option<String>,
	/// custom embed info for a template
	#[serde(default)]
	pub embed: Option<EmbedMetadata>,
	/// The page's custom scripts, if any.
	#[serde(default)]
	pub scripts: Vec<String>,
	/// the page's custom styles, if any.
	#[serde(default)]
	pub styles: Vec<String>,
	/// The extra stuff to run for the page, if any.
	#[serde(default)]
	pub extra: Option<ExtraData>,
}

/// Struct containing information about the site.
#[derive(Debug)]
pub struct Site {
	/// The path to the site.
	pub site_path: PathBuf,
	/// The site's configuration.
	pub config: SiteConfig,
	/// An index of available pages.
	pub page_index: HashMap<String, PathBuf>,
}

impl Site {
	/// Creates a new site from the given path.
	pub fn new(site_path: &Path) -> eyre::Result<Self> {
		let config = SiteConfig::read(site_path)?;

		let mut page_index = HashMap::new();
		let pages_path = site_path.join(PAGES_PATH);
		for entry in WalkDir::new(&pages_path).into_iter() {
			let entry = entry.wrap_err("Failed to read page entry")?;
			let path = entry.path();

			if let Some(ext) = path.extension() {
				if ext == "md" && entry.file_type().is_file() {
					page_index.insert(
						path.strip_prefix(&pages_path)
							.wrap_err("This really shouldn't have happened")?
							.with_extension("")
							.to_string_lossy()
							.to_string(),
						path.to_owned(),
					);
				}
			}
		}

		Ok(Self {
			site_path: site_path.to_owned(),
			config,
			page_index,
		})
	}

	/// Builds the site once.
	pub fn build_once(self) -> eyre::Result<()> {
		let builder = SiteBuilder::new(self, false)?.prepare()?;

		builder.site.build_all_pages(&builder)?;
		builder.build_sass()?;

		for (_source_path, config) in builder.site.config.resources.iter() {
			let mut res_builder = resource::ResourceBuilder::new(config.clone());
			res_builder.load_all(&builder)?;
			res_builder.build_all(&builder)?;
		}

		Ok(())
	}

	/// Helper method to build all available pages.
	fn build_all_pages(&self, builder: &SiteBuilder) -> eyre::Result<()> {
		self.page_index
			.keys()
			.par_bridge()
			.try_for_each(|page_name| builder.build_page(page_name))?;

		Ok(())
	}
}
