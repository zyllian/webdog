---
title: templates
template: docs.tera
---

# webdog templates

in webdog, [Tera](https://keats.github.io/tera/) templates are used as the backbone for your site. as used in webdog, your Tera templates are essentially just an extension on top of html.

when you create your site, a file at `templates/base.tera` will be created, acting as the default base template for all pages.

## base template blocks

all base templates should include a block named `content`. this will be used in other templates, particularly resource templates, to render to the correct location on the page.

## template data

all templates are given the following variables:

### `title`

the page's full title, as shown in the browser tab.

### `page`

the main html content for the page to be rendered. can be rendered anywhere you choose, for instance:

```html
<main class="page">
  {% block content %}{{ page | safe }}{% endblock content %}
</main>
```
