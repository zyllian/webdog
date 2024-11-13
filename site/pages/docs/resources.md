---
title: resources
template: docs.tera
---

# resources

resources are a highly configurable aspect of webdog used for taggable things which generally follow a similar layout across entries. for instance, a blog or photo album.

for instructions on how to add a resource via the cli, see the [commands page](./commands).

## resource config

each resource added has its own configuration inside of the main site config file with the following properties:

### `source_path`

the source path for where the resources of this type are located, relative to `<site_path>/resources/`.

### `output_path_resources`

the path prefix for a resource, i.e. "blog" for `/blog/<post id>` or "i" for `/i/<image id>`. can be shared with `output_path_lists` for them to have the same prefix.

### `output_path_lists`

the path prefix for other resource pages, like the overview or tags. i.e. "blog" for `/blog/tags/<tag>` or "images" for `/images/tags/<tag>`. can be shared with `output_path_resources` for them to have the same prefix.

### `resource_template`

the template to use for the main page of a resource item.

this template is provided the resource's properties (defined below) as variables. this should change to be `r.<property>` before the first crates.io release to avoid conflicts with page properties.

### `resource_list_template`

the template to use for a list of resources.

this template is provided with the following properties:

#### `previous`

if this property exists, it can be used as `./{{previous}}` to get a link to the previous list page.

#### `next`

if this property exists, it can be used as `./{{next}}` to get a link to the next list page.

#### `resources`

a array of the resources to be rendered on this page. each resource in the array contains the resource properties as defined below.

### `tag_list_template`

the template to use for a list of tags assigned to the resource.

this template is provided with the following properties:

#### `title`

the title of the tag list.

this property will be renamed before the crates.io release to fix its conflict with page titles.

#### `links`

array of the links available for this list, containing the following properties:

##### `link`

the link to actually use for the link.

##### `title`

the link's title.

### `rss_template`

the template used to render the resource type's rss feed's html content.

this template is provided a single resource's properties as its properties.

### `rss_title`

the title for the resource type's rss feed.

### `rss_description`

the description for the resource type's rss feed.

### `list_title`

the title to use for a list of resources of this type.

### `tag_list_title`

the title to use for a list of tags for this resource t ype.

### `resource_name_plural`

the name of this resource type if it is plural.

### `resources_per_page`

how many resources of this type to display per page when content is paginated.

## defining a resource

resources are made up of markdown files with yaml front matter. for instance:

```md
---
title: resource title
timestamp: 2024-11-13T00:55:46.888967374Z
tags: [first tag, second tag]
---

# hiiiiiii :3
```

the front matter metadata may contain the following properties:

### `title`

the resource's title, as displayed in the browser tab.

### `timestamp`

the timestamp to use as the resource's publishing time. probably shouldn't be altered after publishing publicly, but it won't break anything.

the timestamp follows the [RFC 3339 format](https://www.rfc-editor.org/rfc/rfc3339). use the `webdog now` command to get a timestamp of the proper format easily.

### `tags`

array of the resource's tags. tags are used to group resources together and are at present **required**.

### `cdn_file` (optional)

special property which will take a relative url and add the cdn prefix as defined in the site config to it.

### `desc` (optional)

property for a resource's short description.

### `draft` (optional)

whether the resource is a draft and should be excluded from normal builds. defaults to false.

### other properties

resources may add extra properties which will get passed to the various resource templates later. simply add the property like it was any other property.
