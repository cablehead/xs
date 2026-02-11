# cross.stream Documentation

The official documentation site for **xs** - an event stream store for personal, local-first use.

Documentation is written in Markdown (`.md`) or MDX (`.mdx`) files in the `src/content/docs/` directory.

## Content Organization (Diataxis)

Documentation follows the [Diataxis](https://diataxis.fr/) framework. Place new
pages in the section that matches their purpose:

| Section           | Purpose                                        |
| :---------------- | :--------------------------------------------- |
| Getting Started   | First-run orientation (install, first stream)   |
| Tutorials         | Learning-oriented walkthroughs                  |
| How-to Guides     | Goal-oriented recipes for specific tasks        |
| Explanation        | Understanding-oriented background and concepts  |
| Reference         | Information-oriented API and topic docs          |

When in doubt, ask: "Is the reader *learning* (tutorial), *doing* (how-to),
*understanding* (explanation), or *looking something up* (reference)?"

## Development Commands

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

