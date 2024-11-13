use std::{
	collections::BTreeMap,
	path::{Path, PathBuf},
};

use eyre::Context;
use itertools::Itertools;
use pulldown_cmark::{Options, Parser};
use rss::{validation::Validate, ChannelBuilder, ItemBuilder};
use serde::{Deserialize, Serialize, Serializer};
use time::{format_description::well_known::Rfc2822, OffsetDateTime};

use crate::{
	builder::SiteBuilder, frontmatter::FrontMatterRequired, link_list::Link, PageMetadata,
};

/// Metadata for resources.
#[derive(Debug, Deserialize, Serialize)]
pub struct ResourceMetadata {
	/// The resource's title.
	pub title: String,
	/// The resource's timestamp.
	#[serde(with = "time::serde::rfc3339")]
	pub timestamp: OffsetDateTime,
	/// The resource's tags.
	pub tags: Vec<String>,
	/// Special field that gets transformed to the full CDN URL for the given path.
	pub cdn_file: Option<String>,
	/// The resource's description, if any.
	pub desc: Option<String>,
	/// Extra resource data not included.
	#[serde(flatten)]
	pub inner: serde_yml::Value,
	/// Whether the resource is a draft. Drafts can be committed without being published to the live site.
	#[serde(default)]
	pub draft: bool,
}

#[derive(Debug, Serialize)]
pub struct ResourceTemplateData<'r> {
	/// The resource's metadata.
	#[serde(flatten)]
	pub resource: &'r FrontMatterRequired<ResourceMetadata>,
	/// The resource's ID.
	pub id: String,
	/// The resource's timestamp. Duplicated to change serialization method.
	#[serde(serialize_with = "ResourceTemplateData::timestamp_formatter")]
	pub timestamp: OffsetDateTime,
}

impl<'r> ResourceTemplateData<'r> {
	fn timestamp_formatter<S>(timestamp: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let out = timestamp
			.format(
				&time::format_description::parse("[weekday], [month repr:long] [day], [year]")
					.expect("Should never fail"),
			)
			.expect("Should never fail");
		serializer.serialize_str(&out)
	}
}

/// struct for adding custom meta content embeds
#[derive(Debug, Serialize, Deserialize)]
pub struct EmbedMetadata {
	pub title: String,
	#[serde(default)]
	pub description: Option<String>,
	#[serde(default)]
	pub image: Option<String>,
	pub theme_color: Option<String>,
	#[serde(default)]
	pub large_image: bool,
}

impl EmbedMetadata {
	/// builds the embed html tags
	pub fn build(self, builder: &SiteBuilder) -> eyre::Result<String> {
		let mut s = format!(
			r#"<meta content="{}" property="og:title"><meta content="{}" property="og:url"><meta content="{}" property="og:site_name">"#,
			self.title, builder.site.config.base_url, builder.site.config.title,
		);
		if let Some(description) = self.description {
			s = format!(r#"{s}<meta content="{description}" property="og:description">"#);
		}
		if let Some(mut image) = self.image {
			if image.starts_with("cdn$") {
				image = builder.site.config.cdn_url(&image[4..])?.to_string();
			}
			s = format!(r#"{s}<meta content="{image}" property="og:image">"#);
		}
		let theme_color = self
			.theme_color
			.as_ref()
			.unwrap_or(&builder.site.config.theme_color);
		s = format!(r#"{s}<meta content="{theme_color}" name="theme-color">"#);
		if self.large_image {
			s = format!(r#"{s}<meta name="twitter:card" content="summary_large_image">"#);
		}

		Ok(s)
	}

	pub fn default_theme_color() -> String {
		"#ffc4fc".to_string()
	}
}

#[derive(Debug, Serialize)]
struct ResourceListTemplateData<'r> {
	resources: Vec<&'r ResourceTemplateData<'r>>,
	tag: Option<&'r str>,
	page: usize,
	page_max: usize,
	previous: Option<usize>,
	next: Option<usize>,
}

/// Config for the resource builder.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceBuilderConfig {
	/// Path to where the resources should be loaded from.
	pub source_path: String,
	/// Path to where the resource pages should be written to.
	pub output_path_resources: String,
	/// Path to where the main list should be written to.
	pub output_path_lists: String,
	/// The template used to render a single resource.
	pub resource_template: String,
	/// The template used to render a list of resources.
	pub resource_list_template: String,
	/// The template used to render the resource's tag pages.
	pub tag_list_template: String,
	/// Template used when rendering the RSS feed.
	pub rss_template: String,
	/// The RSS feed's title.
	pub rss_title: String,
	/// The description for the RSS feed.
	pub rss_description: String,
	/// Title for the main list of resources.
	pub list_title: String,
	/// Title for the page containing a list of tags.
	pub tag_list_title: String,
	/// Name for the resource type in plural.
	pub resource_name_plural: String,
	/// The number of resources to display on a single page.
	pub resources_per_page: usize,
}

/// Helper to genericize resource building.
#[derive(Debug, Default)]
pub struct ResourceBuilder {
	/// The builder's config.
	pub config: ResourceBuilderConfig,
	/// The currently loaded resource metadata.
	pub loaded_metadata: Vec<(String, FrontMatterRequired<ResourceMetadata>)>,
}

impl ResourceBuilder {
	/// Creates a new resource builder.
	pub fn new(config: ResourceBuilderConfig) -> Self {
		Self {
			config,
			loaded_metadata: Default::default(),
		}
	}

	/// Gets a resource's ID from its path.
	fn get_id(path: &Path) -> String {
		path.with_extension("")
			.file_name()
			.expect("Should never fail")
			.to_string_lossy()
			.into_owned()
	}

	/// Loads resource metadata from the given path.
	fn load(
		builder: &SiteBuilder,
		path: &Path,
	) -> eyre::Result<(String, FrontMatterRequired<ResourceMetadata>)> {
		let id = Self::get_id(path);

		let input = std::fs::read_to_string(path)?;
		let mut page = FrontMatterRequired::<ResourceMetadata>::parse(input)
			.wrap_err_with(|| eyre::eyre!("Failed to parse resource front matter"))?;

		let parser = Parser::new_ext(&page.content, Options::all());
		let mut html = String::new();
		pulldown_cmark::html::push_html(&mut html, parser);
		*page.content_mut() = html;

		let data = page.data_mut();
		if let Some(cdn_file) = &data.cdn_file {
			data.cdn_file = Some(builder.site.config.cdn_url(cdn_file)?.to_string());
		}

		Ok((id, page))
	}

	/// Loads all resource metadata from the given config.
	pub fn load_all(&mut self, builder: &SiteBuilder) -> eyre::Result<()> {
		let lmd = &mut self.loaded_metadata;
		lmd.clear();
		for e in builder
			.site
			.site_path
			.join(crate::RESOURCES_PATH)
			.join(&self.config.source_path)
			.read_dir()?
		{
			let p = e?.path();
			if let Some("md") = p.extension().and_then(|e| e.to_str()) {
				let (id, metadata) = Self::load(builder, &p)?;
				if !builder.serving && metadata.data.as_ref().expect("should never fail").draft {
					continue;
				}
				lmd.push((id, metadata));
			}
		}
		lmd.sort_by(|a, b| {
			b.1.data
				.as_ref()
				.expect("should never fail")
				.timestamp
				.cmp(&a.1.data.as_ref().expect("should never fail").timestamp)
		});
		Ok(())
	}

	/// Gets a resource's build path.
	fn build_path(&self, base_path: &Path, id: &str) -> PathBuf {
		base_path
			.join(&self.config.output_path_resources)
			.join(id)
			.with_extension("html")
	}

	/// Builds a single resource page.
	fn build(
		&self,
		builder: &SiteBuilder,
		id: String,
		resource: &FrontMatterRequired<ResourceMetadata>,
	) -> eyre::Result<()> {
		let out_path = self.build_path(&builder.build_path, &id);

		let data = resource.data();
		let out = builder.build_page_raw(
			PageMetadata {
				template: Some(self.config.resource_template.clone()),
				title: Some(data.title.clone()),
				embed: Some(EmbedMetadata {
					title: data.title.clone(),
					description: data.desc.clone(),
					image: if let Some(cdn_file) = &data.cdn_file {
						Some(builder.site.config.cdn_url(cdn_file)?.to_string())
					} else {
						None
					},
					theme_color: None,
					large_image: true,
				}),
				..Default::default()
			},
			"",
			ResourceTemplateData {
				resource,
				id,
				timestamp: resource.data.as_ref().expect("should never fail").timestamp,
			},
		)?;
		std::fs::write(out_path, out)?;

		Ok(())
	}

	pub fn build_all(&self, builder: &SiteBuilder) -> eyre::Result<()> {
		let out_short = builder.build_path.join(&self.config.output_path_resources);
		let out_long = builder.build_path.join(&self.config.output_path_lists);

		if !out_short.exists() {
			std::fs::create_dir_all(&out_short)?;
		}
		if !out_long.exists() {
			std::fs::create_dir_all(&out_long)?;
		}

		let lmd = &self.loaded_metadata;

		for (id, resource) in lmd.iter() {
			self.build(builder, id.clone(), resource)?;
		}

		let mut data = Vec::with_capacity(lmd.len());
		for (id, resource) in lmd.iter() {
			data.push(ResourceTemplateData {
				resource,
				id: id.clone(),
				timestamp: resource.data().timestamp,
			});
		}

		fn build_list(
			builder: &SiteBuilder,
			config: &ResourceBuilderConfig,
			list: Vec<&ResourceTemplateData>,
			title: &str,
			tag: Option<&str>,
			out_path: &Path,
			items_per_page: usize,
		) -> eyre::Result<()> {
			if !out_path.exists() {
				std::fs::create_dir_all(out_path)?;
			}

			let page_max = list.len() / items_per_page + (list.len() % items_per_page).min(1);
			let mut previous = None;
			let mut next;
			for (page, iter) in list.iter().chunks(items_per_page).into_iter().enumerate() {
				next = (page + 1 != page_max).then_some(page + 2);
				let out = builder.build_page_raw(
					PageMetadata {
						template: Some(config.resource_list_template.clone()),
						title: Some(title.to_owned()),
						..Default::default()
					},
					"",
					ResourceListTemplateData {
						resources: iter.copied().collect(),
						tag,
						page: page + 1,
						page_max,
						previous,
						next,
					},
				)?;
				if page == 0 {
					std::fs::write(out_path.join("index.html"), &out)?;
				}
				std::fs::write(
					out_path.join((page + 1).to_string()).with_extension("html"),
					out,
				)?;
				previous = Some(page + 1);
			}

			Ok(())
		}

		// Build main list of resources
		build_list(
			builder,
			&self.config,
			data.iter().collect(),
			&self.config.list_title,
			None,
			&out_long,
			self.config.resources_per_page,
		)?;

		// Build resource lists by tag
		let mut tags: BTreeMap<String, Vec<&ResourceTemplateData>> = BTreeMap::new();
		for resource in &data {
			for tag in resource.resource.data().tags.iter().cloned() {
				tags.entry(tag).or_default().push(resource);
			}
		}

		// Build list of tags
		{
			let links = tags
				.iter()
				.map(|(tag, data)| {
					let count = data.len();
					(
						Link::new(
							format!("/{}/tag/{tag}/", self.config.output_path_resources),
							format!("{tag} ({count})"),
						),
						count,
					)
				})
				.sorted_by(|(_, a), (_, b)| b.cmp(a))
				.map(|(l, _)| l)
				.collect();
			let out = crate::link_list::render_basic_link_list(
				builder,
				&self.config.tag_list_template,
				links,
				&self.config.tag_list_title,
			)?;
			std::fs::write(out_short.join("tags.html"), out)?;
		}

		for (tag, data) in tags {
			build_list(
				builder,
				&self.config,
				data,
				&format!("{} tagged {tag}", self.config.resource_name_plural),
				Some(tag.as_str()),
				&out_short.join("tag").join(&tag),
				self.config.resources_per_page,
			)?;
		}

		// Build RSS feed
		let mut items = Vec::with_capacity(data.len());
		for resource in data {
			items.push(
				ItemBuilder::default()
					.title(Some(resource.resource.data().title.to_owned()))
					.link(Some(
						builder
							.site
							.config
							.base_url
							.join(&format!(
								"{}/{}",
								self.config.output_path_resources, resource.id
							))?
							.to_string(),
					))
					.description(resource.resource.data().desc.clone())
					.pub_date(Some(resource.timestamp.format(&Rfc2822)?))
					.content(Some(builder.tera.render(
						&self.config.rss_template,
						&tera::Context::from_serialize(resource)?,
					)?))
					.build(),
			)
		}

		let channel = ChannelBuilder::default()
			.title(self.config.rss_title.clone())
			.link(
				builder
					.site
					.config
					.base_url
					.join(&format!("{}/", self.config.output_path_lists))
					.expect("Should never fail"),
			)
			.description(self.config.rss_description.clone())
			.last_build_date(Some(OffsetDateTime::now_utc().format(&Rfc2822)?))
			.items(items)
			.build();
		channel.validate().wrap_err("Failed to validate RSS feed")?;
		let out = channel.to_string();
		std::fs::write(out_long.join("rss.xml"), out)?;

		Ok(())
	}
}
