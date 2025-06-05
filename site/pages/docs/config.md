---
title: config
template: docs.tera
---

# webdog configuration

your webdog site must have a `config.yaml` file in its root with the following keys:

## `base_url`

simply the site's base url.

## `title`

the site's base title applied to all pages.

## `description`

presently unused field, set this to any string you like.

## `theme_color`

the default theme color to use for embed metadata.

## `build`

the directory for the site to build to. defaults to `<site-path>/build` if not specified.

**!!WARNING!! this may not function properly, it is untested in any version and should not be relied upon in future versions**

## `sass_styles`

list of sass/scss stylesheets in the site's `sass` directory to treat as root stylesheets and build.

## `cdn_url`

base url for the various cdn url transformation features of webdog.

## `webdog_path`

optional custom path for webdog's static resources.

## `code_theme`

the theme to use for code blocks. valid options: `base16-ocean.dark`, `base16-eighties.dark`, `base16-mocha.dark`, `base16-ocean.light`, `InspiredGitHub`, `Solarized (dark)`, and `Solarized (light)`

## `resources`

configuration information for your site's resource types. must be present, even if no resources have been added. see <a href="resources">resources documentation</a> for more info.
