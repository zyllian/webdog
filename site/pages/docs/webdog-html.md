---
title: webdog html
template: docs.tera
---

# webdog html

webdog adds some extensions to html to make building your site easier.

## `wd-partial`

any template can be used as a partial in another template or page.

in a template like this:

```html
<p>this is a partial.</p>
<p>the hi argument is {{ userdata.hi }}</p>
<p>the hello argument is {{ userdata.hello }}</p>
<div>
  <p>and here's the inner content:
  {{ page | safe }}
</div>
```

simply include the `wd-partial` html tag like so:

```html
<wd-partial t="template-to-use.tera" hello="hi" hi="hello">
  hiiiiiiiiiii~
</wd-partial>
```

a `wd-partial` tag consists of the `t` attribute to determine the template and any number of additional arguments, which are passed on to the partial template.
