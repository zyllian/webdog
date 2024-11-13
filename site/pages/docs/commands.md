---
title: commands
template: docs.tera
---

# webdog commands

global options:

```
--site <SITE_PATH>  The path to the site [default: .]
-h, --help          Print help
```

## `webdog create`

```
Create a new webdog site

Usage: webdog create [OPTIONS] <BASE_URL> <TITLE>

Arguments:
  <BASE_URL>  The site's base URL
  <TITLE>     The site's title

Options:
      --cdn-url <CDN_URL>      The site's CDN URL. (defaults to the base URL)
```

## `webdog build`

```
Builds the site

Usage: webdog build [OPTIONS]
```

## `webdog serve`

```
Serves the site for locally viewing edits made before publishing

Usage: webdog serve [OPTIONS]

Options:
      --ip <IP>                The IP address to bind to [default: 127.0.0.1]
  -p, --port <PORT>            The port to bind to [default: 8080]
```

## `webdog now`

```
Helper to get the current timestamp

Usage: webdog now [OPTIONS]
```

## `webdog resource create`

```
Creates a new resource type

Usage: webdog resource create [OPTIONS] <ID> <NAME> <PLURAL>

Arguments:
  <ID>      The resource type's ID
  <NAME>    The name of the resource type to create
  <PLURAL>  The name of the resource type, but plural

Options:
      --no-rss       Whether to skip enabling RSS for this resource or not
```

## `webdog page new`

```
Creates a new standard page

Usage: webdog page new [OPTIONS] <ID> [TITLE]

Arguments:
  <ID>     The page's ID
  [TITLE]  The page's title

Options:
      --template <TEMPLATE>    The page's base template if using one other than the default
```

## `webdog new`

```
Creates a new resource of the given type

Usage: webdog new [OPTIONS] <RESOURCE_TYPE> <ID> <TITLE>

Arguments:
  <RESOURCE_TYPE>  The type of resource to create
  <ID>             The resource's ID
  <TITLE>          The resource's title

Options:
  -t, --tag <TAGS>                 The resource's tags
  -d, --description <DESCRIPTION>  The resource's description
      --skip-draft                 Whether to skip setting the resource as a draft or not
```
