use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use include_dir::{include_dir, Dir};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use url::Url;
use webdog::{
	frontmatter::FrontMatter,
	resource::{ResourceBuilderConfig, ResourceMetadata},
	PageMetadata, Site, SiteConfig,
};

/// The default project to use when creating a new one, embedded into the binary.
static DEFAULT_PROJECT: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/embedded/default_site");
/// The default resource template, embedded into the binary.
static DEFAULT_RESOURCE_TEMPLATES: Dir =
	include_dir!("$CARGO_MANIFEST_DIR/src/embedded/resource-template");

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Commands,

	/// The path to the site.
	#[arg(global = true, long, default_value = ".")]
	site_path: PathBuf,
}

#[derive(Debug, Subcommand)]
enum Commands {
	/// Create a new webdog site.
	Create {
		/// The site's base URL.
		base_url: Url,
		/// The site's title.
		title: String,
		/// The site's CDN URL. (defaults to the base URL)
		#[arg(long)]
		cdn_url: Option<Url>,
	},
	/// Builds the site.
	Build {},
	/// Serves the site for locally viewing edits made before publishing.
	#[cfg(feature = "serve")]
	Serve {
		/// The IP address to bind to.
		#[arg(long, default_value = "127.0.0.1")]
		ip: String,
		/// The port to bind to.
		#[arg(short, long, default_value = "8080")]
		port: u16,
	},
	/// Helper to get the current timestamp.
	Now,
	/// For dealing with site resources.
	Resource {
		#[clap(subcommand)]
		command: ResourceCommands,
	},
	/// For dealing with standard site pages.
	Page {
		#[clap(subcommand)]
		command: PageCommands,
	},
	/// Creates a new resource of the given type.
	New {
		/// The type of resource to create.
		resource_type: String,
		/// The resource's ID.
		id: String,
		/// The resource's title.
		title: String,
		/// The resource's tags.
		#[arg(short, long = "tag")]
		tags: Vec<String>,
		/// The resource's description.
		#[arg(short, long)]
		description: Option<String>,
		/// Whether to skip setting the resource as a draft or not.
		#[arg(long, default_value = "false")]
		skip_draft: bool,
	},
}

#[derive(Debug, Subcommand)]
enum ResourceCommands {
	/// Creates a new resource type.
	Create {
		/// The resource type's ID.
		id: String,
		/// The name of the resource type to create.
		name: String,
		/// The name of the resource type, but plural.
		plural: String,
	},
}

#[derive(Debug, Subcommand)]
enum PageCommands {
	/// Creates a new standard page.
	New {
		/// The page's ID.
		id: String,
		/// The page's title.
		title: Option<String>,
		/// The page's base template if using one other than the default.
		#[arg(long)]
		template: Option<String>,
	},
}

fn main() -> eyre::Result<()> {
	#[cfg(feature = "color-eyre")]
	color_eyre::install()?;

	let cli = Cli::parse();

	let site = || -> eyre::Result<Site> { Site::new(&Path::new(&cli.site_path).canonicalize()?) };

	match cli.command {
		Commands::Create {
			base_url,
			cdn_url,
			title,
		} => {
			if cli.site_path.exists() {
				eprintln!("content exists in the given path! canceling!");
				return Ok(());
			}
			std::fs::create_dir_all(&cli.site_path)?;
			let config = SiteConfig::new(base_url.clone(), cdn_url.unwrap_or(base_url), title);
			std::fs::write(
				cli.site_path.join(SiteConfig::FILENAME),
				serde_yml::to_string(&config)?,
			)?;
			DEFAULT_PROJECT.extract(&cli.site_path)?;
			std::fs::create_dir(cli.site_path.join(webdog::ROOT_PATH))?;

			println!(
				"Base site created at {:?}! Ready for editing, woof!",
				cli.site_path
			);

			Ok(())
		}
		Commands::Build {} => {
			println!("Building site...");
			let now = std::time::Instant::now();
			site()?.build_once()?;
			println!("Build completed in {:?}", now.elapsed());
			Ok(())
		}
		#[cfg(feature = "serve")]
		Commands::Serve { ip, port } => {
			let site = site()?;
			let rt = tokio::runtime::Runtime::new()?;
			rt.block_on(async move { site.serve(&format!("{}:{}", ip, port)).await })
		}
		Commands::Now => {
			let time = OffsetDateTime::now_utc();
			println!("{}", time.format(&Rfc3339)?);
			Ok(())
		}
		Commands::Resource { command } => match command {
			ResourceCommands::Create { id, name, plural } => {
				let config_path = cli.site_path.join(SiteConfig::FILENAME);
				let mut config = SiteConfig::read(&cli.site_path)?;
				if config.resources.contains_key(&id) {
					eprintln!("resource type {id} already exists, canceling");
					return Ok(());
				}

				let resource_template_path = cli.site_path.join(webdog::TEMPLATES_PATH).join(&id);
				if resource_template_path.exists() {
					eprintln!(
						"path for resource already exists at {resource_template_path:?}, canceling"
					);
					return Ok(());
				}
				std::fs::create_dir_all(&resource_template_path)?;
				for file in DEFAULT_RESOURCE_TEMPLATES.files() {
					let resource_path = resource_template_path.join(file.path());
					if let Some(contents) = file.contents_utf8() {
						let mut contents = contents.to_owned();
						contents = contents.replace("!!RESOURCE_TYPE!!", &id);
						contents = contents.replace("!!RESOURCE_NAME!!", &name);
						contents =
							contents.replace("!!RESOURCE_NAME_LOWERCASE!!", &name.to_lowercase());
						contents = contents.replace("!!RESOURCE_NAME_PLURAL!!", &plural);
						contents = contents
							.replace("!!RESOURCE_NAME_PLURAL_LOWERCASE!!", &plural.to_lowercase());
						std::fs::write(resource_path, contents)?;
					} else {
						std::fs::write(resource_path, file.contents())?;
					}
				}

				let resource_config = ResourceBuilderConfig {
					source_path: id.clone(),
					output_path_short: id.clone(),
					output_path_long: id.clone(),
					resource_template: format!("{id}/resource.tera"),
					resource_list_template: format!("{id}/list.tera"),
					tag_list_template: "basic-link-list.tera".to_string(),
					rss_template: format!("{id}/rss.tera"),
					rss_title: id.clone(),
					rss_description: Default::default(),
					list_title: name.clone(),
					tag_list_title: format!("{name} tags"),
					resource_name_plural: plural,
					resources_per_page: 3,
				};

				config.resources.insert(id.clone(), resource_config);

				std::fs::write(config_path, serde_yml::to_string(&config)?)?;

				let resource_path = cli.site_path.join(webdog::RESOURCES_PATH).join(&id);
				std::fs::create_dir_all(&resource_path)?;

				create_resource(
					&resource_path.join("first.md"),
					&ResourceMetadata {
						title: format!("First {name}"),
						timestamp: OffsetDateTime::now_utc(),
						tags: vec!["first".to_string()],
						cdn_file: None,
						desc: Some(format!("This is the first {name} :)")),
						inner: serde_yml::Value::Null,
						draft: true,
					},
				)?;

				println!("Created the new resource type {id}! The first resource of this time is available at {:?}.", resource_path);

				Ok(())
			}
		},
		Commands::Page { command } => match command {
			PageCommands::New {
				id,
				title,
				template,
			} => {
				let page_path = cli
					.site_path
					.join(webdog::PAGES_PATH)
					.join(&id)
					.with_extension("md");
				if page_path.exists() {
					eprintln!("page already exists!");
					return Ok(());
				}
				let fm = FrontMatter {
					content: "new page :)".to_string(),
					data: Some(PageMetadata {
						title,
						template,
						..Default::default()
					}),
				};
				std::fs::write(&page_path, fm.format()?)?;

				println!("Page created! Edit at {:?}.", page_path);

				Ok(())
			}
		},
		Commands::New {
			resource_type,
			id,
			title,
			tags,
			description,
			skip_draft,
		} => {
			let config = SiteConfig::read(&cli.site_path)?;
			if let Some(resource) = config.resources.get(&resource_type) {
				let resource_path = cli
					.site_path
					.join(webdog::RESOURCES_PATH)
					.join(&resource.source_path)
					.join(&id)
					.with_extension("md");

				if resource_path.exists() {
					eprintln!(
						"A {resource_type} resource of the ID {id} already exists, canceling!"
					);
					return Ok(());
				}

				create_resource(
					&resource_path,
					&ResourceMetadata {
						title,
						timestamp: OffsetDateTime::now_utc(),
						tags,
						cdn_file: None,
						desc: description,
						inner: serde_yml::Value::Null,
						draft: !skip_draft,
					},
				)?;

				println!(
					"Created the new {resource_type} resource {id}! Available at {:?}",
					resource_path
				);
			} else {
				eprintln!("no resource of type {resource_type}, canceling");
			}
			Ok(())
		}
	}
}

/// Creates a new resource from the given metadata.
fn create_resource(resource_path: &Path, metadata: &ResourceMetadata) -> eyre::Result<()> {
	std::fs::write(
		resource_path,
		FrontMatter {
			content: "hello world :)".to_string(),
			data: Some(metadata),
		}
		.format()?,
	)?;
	Ok(())
}
