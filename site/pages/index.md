---
extra:
  name: resource-list-outside
  template: extras/index-injection.tera
  resource: blog
  count: 3
---

# webdog

welcome to webdog, the static site generator fit for a dog :3

if you have [rust](https://rust-lang.org) installed, all you need to do to install webdog is run the following command:

```sh
cargo install webdog --git https://github.com/zyllian/webdog
```

then you can make your first webdog site!

```sh
webdog create https://example.com "My First Site!" --site-path my-site
cd my-site
webdog serve # your site is now running at http://127.0.0.1:8080 🥳
```

and from there, you can editing your site and adding pages!

```sh
webdog page new my-first-page "My First Page"
```
