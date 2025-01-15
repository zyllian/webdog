# webdog

webdog, the static site generator fit for a dog :3

```sh
cargo install webdog
```

after installing, you can create your first site:

```sh
webdog create https://example.com "My First Site!" --site my-site
cd my-site
webdog serve # your site is now running at http://127.0.0.1:8080 🥳
```

from there, you can start editing your site and adding pages or [more advanced things](https://webdog.zyl.gay/docs/)

```sh
webdog page new my-first-page "My First Page"
```
