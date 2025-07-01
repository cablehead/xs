# xs Documentation

This folder contains the [Starlight](https://starlight.astro.build/) site that powers the
project documentation. It is built with [Astro](https://astro.build/) and uses
Deno for development and building.

## Requirements

- [Deno](https://deno.com/runtime) 1.38 or newer

Deno can run the Astro CLI directly from npm without installing a local
`node_modules` directory.

## Development

Start a local server from the `docs` directory:

```sh
deno run -A npm:astro dev
```

The site will be available at `http://localhost:4321`.

## Building

Generate the static site:

```sh
deno run -A npm:astro build
```

Preview the result with:

```sh
deno run -A npm:astro preview
```

All generated files will be placed in `docs/dist/`.

## Project layout

```
docs/
  public/        # static files copied as-is
  src/
    assets/      # images and other assets
    content/     # documentation pages
    content.config.ts
  astro.config.mjs
  package.json
  tsconfig.json
```

For more details on customizing Starlight, see the
[official Starlight documentation](https://starlight.astro.build/).
