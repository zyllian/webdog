---
title: templates
template: docs.tera
---

# webdog templates

in webdog, [Tera](https://keats.github.io/tera/) templates are used as the backbone for your site. as used in webdog, your Tera templates are essentially just an extension on top of html.

when you create your site, a file at `templates/base.tera` will be created, acting as the default base template for all pages.

## base template variables

a great deal of information is given to templates used as bases, such as:

### `title`

the page's title. used to set the page's title in the head, though this will no longer be needed in a future update.

### `head`

html code which must be written to the head using the safe filter. will be removed in a future update.

### `scripts`

a list of extra scripts to apply to the page. will be removed in a future update.

### `styles`

a list of extra stylesheets to apply to the page. will be removed in a future update.

### `page`

html code for the page. must be rendered to the page using the safe filter.

## base template blocks

all base templates should include a block named `content`. this will be used in other templates, particularly resource templates, to render to the correct location on the page.

## template data

all templates are given the following variables:

### `title`

the page's title. this should be rendered in the `<head>` html element like so:

```html
<head>
  <title>{{ title }}</title>
</head>
```

this variable will no longer be required to set the page title before the first crates.io release.

### `head`

the page's extra head data. this should be rendered in the `<head>` html element like so:

```html
<head>
  {{ head | safe }}
</head>
```

this variable will be removed before the first crates.io release.

### `scripts`

the page's extra scripts. this should be rendered in the `<head>` html element like so:

```html
<head>
  {% for script in scripts %}
  <script type="text/javascript" src="{{script}}" defer></script>
  {% endfor %}
</head>
```

this variable will be removed before the first crates.io release.

### `styles`

the page's extra stylesheets. this should be rendered in the `<head>` html element like so:

```html
<head>
  {% for style in styles %}
  <link rel="stylesheet" href="/styles/{{style}}" />
  {% endfor %}
</head>
```

this variable will be removed before the first crates.io release.

### `page`

the main html content for the page to be rendered. can be rendered anywhere you choose, for instance:

```html
<main class="page">
  {% block content %}{{ page | safe }}{% endblock content %}
</main>
```
