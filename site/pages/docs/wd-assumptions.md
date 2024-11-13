---
title: webdog assumptions
template: docs.tera
---

# webdog assumptions

although webdog can be used to make many different kinds of static website, there are some assumptions it makes about your site's layout which must be followed.

- there must be a template named "base.tera". pages without a template will assume this is their base template.

- there must be a page named "404.md". this page will be used if a page can't be found.

- other things i'm sure i'm forgetting
