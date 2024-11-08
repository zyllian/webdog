//! Module containing code to serve the dev site.

use std::{
	collections::HashMap,
	net::SocketAddr,
	path::{Path, PathBuf},
	sync::{Arc, Mutex},
};

use eyre::Context;
use futures::SinkExt;
use hotwatch::{EventKind, Hotwatch};
use warp::{
	hyper::StatusCode,
	path::FullPath,
	reply::Response,
	ws::{Message, WebSocket},
	Filter,
};

use crate::{
	resource::RESOURCES_PATH, Site, SiteBuilder, PAGES_PATH, ROOT_PATH, SASS_PATH, TEMPLATES_PATH,
};

/// Helper to get the "name" of a path.
fn get_name(path: &Path) -> (PathBuf, String) {
	let name = path.with_extension("");
	let name_str = name
		.display()
		.to_string()
		.replace(std::path::MAIN_SEPARATOR, "/");
	(name, name_str)
}

fn with_build_path(
	build_path: PathBuf,
) -> impl Filter<Extract = (PathBuf,), Error = std::convert::Infallible> + Clone {
	warp::any().map(move || build_path.clone())
}

/// Helper to make a path relative.
fn rel(path: &Path, prefix: &Path) -> Result<PathBuf, std::path::StripPrefixError> {
	Ok(path.strip_prefix(prefix)?.to_owned())
}

/// Helper to build resources in the case of creation or removal.
fn build_resources(builder: &mut SiteBuilder, path: &Path) -> eyre::Result<()> {
	let paths: Vec<_> = builder
		.resource_builders
		.values()
		.map(|b| {
			(
				b.config.source_path.clone(),
				path.strip_prefix(&b.config.source_path),
			)
		})
		.filter_map(|(p, v)| v.ok().map(|v| (p, v)))
		.collect();
	if paths.len() > 1 {
		todo!("handle more than one possible match");
	}
	if let Some((prefix, _path)) = paths.first() {
		// HACK: this could get very inefficient with a larger number of resources. should definitely optimize
		builder.reload_resource_builder(prefix)?;
		builder.build_resources(prefix)?;
	}
	Ok(())
}

/// Creates or updates a resource.
fn create(
	builder: &mut SiteBuilder,
	path: &Path,
	relative_path: &Path,
	build: bool,
) -> eyre::Result<()> {
	if path.is_dir() {
		return Ok(());
	}
	println!("{relative_path:?}");
	if let Ok(page_path) = relative_path.strip_prefix(PAGES_PATH) {
		let (_page_name, page_name_str) = get_name(page_path);

		builder
			.site
			.page_index
			.insert(page_name_str.clone(), path.to_owned());
		if build {
			builder.build_page(&page_name_str)?;
		}
	} else if let Ok(_template_path) = relative_path.strip_prefix(TEMPLATES_PATH) {
		builder.tera.full_reload()?;
		if build {
			builder.site.build_all_pages(builder)?;
			builder.build_all_resources()?;
		}
	} else if relative_path.display().to_string() == "config.yaml" {
		let new_config = serde_yml::from_str(&std::fs::read_to_string(path)?)?;
		builder.site.config = new_config;
		builder.reload()?;
		builder.site.build_all_pages(builder)?;
	} else if let Ok(_sass_path) = relative_path.strip_prefix(SASS_PATH) {
		if build {
			builder.build_sass().wrap_err("Failed to rebuild Sass")?;
		}
	} else if let Ok(root_path) = relative_path.strip_prefix(ROOT_PATH) {
		std::fs::copy(path, builder.build_path.join(root_path))?;
	} else if let Ok(resources_path) = relative_path.strip_prefix(RESOURCES_PATH) {
		build_resources(builder, resources_path)?;
	}

	Ok(())
}

/// Removes an existing resource.
fn remove(builder: &mut SiteBuilder, path: &Path, relative_path: &Path) -> eyre::Result<()> {
	if path.is_dir() {
		return Ok(());
	}
	if let Ok(page_path) = relative_path.strip_prefix(PAGES_PATH) {
		let (page_name, page_name_str) = get_name(page_path);

		builder.site.page_index.remove(&page_name_str);
		std::fs::remove_file(builder.build_path.join(page_name.with_extension("html")))
			.with_context(|| format!("Failed to remove page at {:?}", path))?;
	} else if let Ok(_template_path) = relative_path.strip_prefix(TEMPLATES_PATH) {
		builder.tera.full_reload()?;
		builder
			.site
			.build_all_pages(builder)
			.wrap_err("Failed to rebuild pages")?;
	} else if let Ok(_sass_path) = relative_path.strip_prefix(SASS_PATH) {
		builder.build_sass().wrap_err("Failed to rebuild Sass")?;
	} else if let Ok(root_path) = relative_path.strip_prefix(ROOT_PATH) {
		std::fs::remove_file(builder.build_path.join(root_path))?;
	} else if let Ok(resources_path) = relative_path.strip_prefix(RESOURCES_PATH) {
		build_resources(builder, resources_path)?;
	}

	Ok(())
}

/// Decides whether to skip a path in the watcher.
fn skip_path(builder: &SiteBuilder, path: &Path) -> bool {
	path.strip_prefix(&builder.build_path).is_ok()
}

impl Site {
	/// Serves the site for development. Don't use this in production.
	pub async fn serve(self) -> eyre::Result<()> {
		let addr = SocketAddr::from(([127, 0, 0, 1], 8080));

		let mut builder = SiteBuilder::new(self, true)?.prepare()?;
		let site = &builder.site;
		let build_path = builder.build_path.clone();

		// Perform initial build
		for page_name in site.page_index.keys() {
			if let Err(e) = builder.build_page(page_name) {
				eprintln!("Failed to build page {}: {}", page_name, e);
			}
		}
		builder.build_sass().wrap_err("Failed to build Sass")?;
		builder
			.build_all_resources()
			.wrap_err("Failed to build resources")?;

		// Map of websocket connections
		let peers: Arc<Mutex<HashMap<SocketAddr, WebSocket>>> =
			Arc::new(Mutex::new(HashMap::new()));

		// Watch for changes to the site
		let mut hotwatch = Hotwatch::new().expect("Hotwatch failed to initialize");
		let hw_peers = peers.clone();
		hotwatch
			.watch(site.site_path.clone(), move |event| {
				let peers = hw_peers.clone();

				let r = (|| {
					let path = event
						.paths
						.first()
						.expect("Should always be at least one path");
					match event.kind {
						EventKind::Modify(_) => {
							if skip_path(&builder, path) {
								Ok(false)
							} else {
								let relp = rel(path, &builder.site.site_path)?;
								if event.paths.len() > 1 {
									let new = event.paths.last().expect("Can never fail");
									let new_rel = rel(new, &builder.site.site_path)?;
									println!("RENAMED - {:?} -> {:?}", relp, new_rel);
									create(&mut builder, new, &new_rel, false)?;
									remove(&mut builder, path, &relp)?;
								} else {
									println!("CHANGED - {:?}", relp);
									create(&mut builder, path, &relp, true)?;
								}
								Ok::<_, eyre::Error>(true)
							}
						}
						EventKind::Create(_) => {
							if skip_path(&builder, path) {
								Ok(false)
							} else {
								let rel = rel(path, &builder.site.site_path)?;
								println!("CREATED - {:?}", rel);
								create(&mut builder, path, &rel, true)?;
								Ok(true)
							}
						}
						EventKind::Remove(_) => {
							if skip_path(&builder, path) {
								Ok(false)
							} else {
								let rel = rel(path, &builder.site.site_path)?;
								println!("REMOVED - {:?}", rel);
								remove(&mut builder, path, &rel)?;
								Ok(true)
							}
						}
						_ => Ok(false),
					}
				})();

				match r {
					Ok(reload) => {
						if reload {
							// Send reload event to connected websockets
							let mut peers = peers.lock().unwrap();
							let mut to_remove = Vec::new();
							for (addr, peer) in peers.iter_mut() {
								let task = async {
									peer.send(Message::text("reload".to_string())).await?;
									Ok::<_, eyre::Error>(())
								};
								to_remove.push(*addr);
								if let Err(e) = futures::executor::block_on(task) {
									eprintln!("{}", e);
								}
							}
							for addr in &to_remove {
								peers.remove(addr);
							}
						}
					}
					Err(e) => eprintln!("Failed to update: {:?}", e),
				}
			})
			.expect("Failed to watch file");

		let routes = warp::any()
			.and(warp::ws())
			.and(warp::filters::addr::remote())
			.and_then(move |ws: warp::ws::Ws, addr| {
				let peers = peers.clone();
				async move {
					// Add websocket connection to peers list
					if let Some(addr) = addr {
						let peers = peers.clone();
						return Ok(ws.on_upgrade(move |websocket| async move {
							peers.lock().unwrap().insert(addr, websocket);
						}));
					}
					Err(warp::reject())
				}
			})
			.or(warp::any().and(warp::get()).and(
				warp::path::full()
					.and(with_build_path(build_path.clone()))
					.and_then(move |path: FullPath, build_path: PathBuf| async move {
						// Serve static files
						let p = &path.as_str()[1..];
						let p = percent_encoding::percent_decode_str(p)
							.decode_utf8()
							.expect("Failed to decode URL");

						if p == "_dev.js" {
							let res =
								Response::new(include_str!("./js/refresh_websocket.js").into());
							return Ok(res);
						}

						let mut p = build_path.join(p.as_ref());

						if !p.exists() {
							p = p.with_extension("html");
						}
						if p.is_dir() {
							p = p.join("index.html");
						}

						if p.exists() {
							let mut res = Response::new("".into());
							match std::fs::read(&p) {
								Ok(body) => {
									*res.body_mut() = body.into();
								}
								Err(e) => {
									eprintln!("{}", e);
									*res.body_mut() = format!("Failed to load: {}", e).into();
									*res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
								}
							}
							return Ok(res);
						}
						Err(warp::reject())
					}),
			))
			.or(warp::any()
				.and(warp::path::full())
				.and_then(move |path: FullPath| {
					let build_path = build_path.clone();
					async move {
						// Handle missing files
						println!("404 - {}", path.as_str());
						let body = match std::fs::read_to_string(build_path.join("404.html")) {
							Ok(body) => body,
							_ => "404 Not Found".to_string(),
						};
						let mut res = Response::new(body.into());
						*res.status_mut() = StatusCode::NOT_FOUND;
						Ok::<_, std::convert::Infallible>(res)
					}
				}));

		println!("Starting server at http://{}", addr);
		warp::serve(routes).run(addr).await;

		Ok(())
	}
}
