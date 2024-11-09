use lol_html::{element, RewriteStrSettings};
use serde::{Deserialize, Serialize};

use crate::{builder::SiteBuilder, resource::ResourceTemplateData};

/// Types of extras.
#[derive(Debug)]
pub enum Extra {
	/// Simply appends to the page within content.
	Basic,
	/// May modify the HTML output in any way.
	HtmlModification(
		fn(page: String, builder: &SiteBuilder, data: &ExtraData) -> eyre::Result<String>,
	),
}

impl Extra {
	/// runs the handler for the extra
	pub fn handle(
		&self,
		page: String,
		builder: &SiteBuilder,
		data: &ExtraData,
	) -> eyre::Result<String> {
		#[derive(Debug, Deserialize)]
		struct BasicData {
			template: String,
		}

		match self {
			Self::Basic => {
				let data: BasicData = serde_yml::from_value(data.inner.clone())?;
				let content = builder.tera.render(&data.template, &tera::Context::new())?;
				append_to(&page, &content, "main.page")
			}
			Self::HtmlModification(f) => (f)(page, builder, data),
		}
	}
}

/// Data for extras.
#[derive(Debug, Deserialize)]
pub struct ExtraData {
	/// The name of the extra to run.
	pub name: String,
	/// The inner data for the extra.
	#[serde(flatten)]
	pub inner: serde_yml::Value,
}

/// Gets the extra for the given value.
pub fn get_extra(extra: &str) -> Option<Extra> {
	match extra {
		"basic" => Some(Extra::Basic),
		"resource-list-outside" => Some(Extra::HtmlModification(resource_list_outside)),
		_ => None,
	}
}

/// Extra to append a tempalte to the page.
fn append_to(page: &str, content: &str, selector: &str) -> eyre::Result<String> {
	Ok(lol_html::rewrite_str(
		page,
		RewriteStrSettings {
			element_content_handlers: vec![element!(selector, move |el| {
				el.append(content, lol_html::html_content::ContentType::Html);
				Ok(())
			})],
			..Default::default()
		},
	)?)
}

/// Extra to add a sidebar to the index page with recent blog posts on it.
fn resource_list_outside(
	page: String,
	builder: &SiteBuilder,
	data: &ExtraData,
) -> eyre::Result<String> {
	#[derive(Debug, Deserialize)]
	struct ResourceListData {
		template: String,
		resource: String,
		count: usize,
	}

	#[derive(Debug, Serialize)]
	struct ResourceListTemplateData<'r> {
		resources: Vec<ResourceTemplateData<'r>>,
	}

	let data: ResourceListData = serde_yml::from_value(data.inner.clone())?;

	let resource_list = builder.tera.render(
		&data.template,
		&tera::Context::from_serialize(ResourceListTemplateData {
			resources: builder
				.resource_builders
				.get(&data.resource)
				.ok_or_else(|| eyre::eyre!("missing resource builder: {}", data.resource))?
				.loaded_metadata
				.iter()
				.take(data.count)
				.map(|(id, v)| ResourceTemplateData {
					resource: v,
					id: id.clone(),
					timestamp: v.data().timestamp,
				})
				.collect(),
		})?,
	)?;

	append_to(&page, &resource_list, "#content")
}
