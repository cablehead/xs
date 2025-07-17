# xs Documentation

[![Built with Starlight](https://astro.badg.es/v2/built-with-starlight/tiny.svg)](https://starlight.astro.build)

This directory contains the documentation site for xs, built with Astro and
Starlight.

## 🚀 Project Structure

Inside of your Astro + Starlight project, you'll see the following folders and
files:

```
.
├── public/
├── src/
│   ├── assets/
│   ├── content/
│   │   ├── docs/
│   └── content.config.ts
├── astro.config.mjs
├── package.json
└── tsconfig.json
```

Starlight looks for `.md` or `.mdx` files in the `src/content/docs/` directory.
Each file is exposed as a route based on its file name.

Images can be added to `src/assets/` and embedded in Markdown with a relative
link.

Static assets, like favicons, can be placed in the `public/` directory.

## 🧞 Development Commands

All commands are run from the `docs/` directory:

| Command             | Action                                           |
| :------------------ | :----------------------------------------------- |
| `npm install`       | Installs dependencies                            |
| `npm run dev`       | Starts local dev server at `localhost:4321/xs/`  |
| `npm run build`     | Build your production site to `./dist/`          |
| `npm run preview`   | Preview your build locally, before deploying     |
| `npm run astro ...` | Run CLI commands like `astro add`, `astro check` |

### Prerequisites

- [Node.js](https://nodejs.org/) (version 18 or higher)
- npm (comes with Node.js)

### Getting Started

1. Navigate to the docs directory:
   ```bash
   cd docs
   ```

2. Install dependencies:
   ```bash
   npm install
   ```

3. Install Playwright system dependencies (required for Mermaid diagrams):
   ```bash
   npx playwright install-deps
   ```

4. Start the development server:
   ```bash
   npm run dev
   ```

The site will be available at `http://localhost:4321/xs/` (or another port if
4321 is in use).

### Building for Production

To build the documentation site for production:

```bash
npm run build
```

This creates a `dist/` directory with the built site ready for deployment.

To preview the production build locally:

```bash
npm run preview
```

### Automated Deployment

The documentation is automatically deployed to GitHub Pages when changes are
pushed to the `main` branch. The deployment workflow:

1. **Trigger**: Push to `main` branch
2. **Build**: Uses Node.js 18 with `npm ci` and `npm run build`
3. **Deploy**: Publishes to GitHub Pages from the `docs/dist` directory

The workflow configuration is in `.github/workflows/deploy-docs.yml`.

## 👀 Want to learn more?

Check out [Starlight’s docs](https://starlight.astro.build/), read
[the Astro documentation](https://docs.astro.build), or jump into the
[Astro Discord server](https://astro.build/chat).
