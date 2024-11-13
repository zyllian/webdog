---
title: pages
template: docs.tera
---

# pages

webdog pages essentially just markdown files with extra features.

to add a new page to your site, just run this command:

```sh
webdog page new <id> <title>
```

you may safely create standard pages in subdirectories by using slashes in your id.

## yaml front matter

all standard webdog pages _may_ include yaml front matter. example:

```md
---
title: page title
---

# page header :)
```

yaml front matter may contain the following options:

### `title`

the page's title to be displayed next to the base title, i.e. `webdog / pages` for this page.

### `template`

the template to use for the page. if not specified, defaults to `base.tera`.

### `embed`

custom embed information for the page, useful for linking on social media.

table containing the following fields:

#### `title`

the custom embed's title.

#### `site_name`

the site name to use for the embed.

#### `description`

the embed's description. optional.

#### `image`

full url to an image to use for the embed. optional.

#### `theme_color`

the theme color to use for the embed. optional, but the default is currently nonconfigurable.

#### `large_image`

used by some sites to determine the size of the image when displayed. `true` or `false`.

### `scripts`

list of extra scripts to include in the page.

### `styles`

list of extra stylesheets to include in the page.

### `extra`

see <a href="extras">extras documentation</a> for more info on this field.

## special features

in addition to standard markdown, webdog comes with some minor additions for ease of use.

### links

links may have commands embedded into them. example:

```md
[example](command$url)
```

currently, the only command is `me`, which adds a `rel="me"` value to the link, useful for certain social media platforms' link verification features.

additionally, any external links will be given `target="_blank"` and `rel="noopener noreferrer"` fields to open in a new tab automatically. this is currently nonconfigurable.
