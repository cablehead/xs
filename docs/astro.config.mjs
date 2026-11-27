// @ts-check
import { defineConfig } from "astro/config";

import starlight from "@astrojs/starlight";

import { rehypeMermaid } from "@beoe/rehype-mermaid";
import solid from "@astrojs/solid-js";
import { unified } from "@astrojs/markdown-remark";
import { pluginCodeOutput } from "./src/utils/custom-code-output-plugin.js";

// https://astro.build/config
export default defineConfig({
  base: "/xs/",
  devToolbar: { enabled: false },

  integrations: [
    solid(),

    starlight({
      title: "cross.stream",

      customCss: [
        "./src/css/custom.css",
      ],

      components: {
        Hero: './src/components/Hero.astro',
      },

      social: [
        { icon: "github", label: "GitHub", href: "https://github.com/cablehead/xs" },
        { icon: "discord", label: "Discord", href: "https://discord.com/invite/YNbScHBHrh" },
      ],

      sidebar: [
        {
          label: "Getting Started",
          items: [{ autogenerate: { directory: "getting-started" } }],
        },

        {
          label: "Tutorials",
          items: [{ autogenerate: { directory: "tutorials" } }],
        },

        {
          label: "How-to Guides",
          items: [{ autogenerate: { directory: "how-to" } }],
        },

        {
          label: "Explanation",
          items: [{ autogenerate: { directory: "explanation" } }],
        },

        {
          label: "Reference",
          items: [{ autogenerate: { directory: "reference" } }],
        },
      ],

      expressiveCode: {
        styleOverrides: { borderRadius: "0.25rem" },
        themes: ["dracula", "rose-pine-dawn"],
        plugins: [pluginCodeOutput()],
      },

      lastUpdated: false,
      credits: true,
    }),
  ],

  markdown: {
    processor: unified({
      rehypePlugins: [
        [
          rehypeMermaid,
          {
            strategy: "file", // alternatively use "data-url"
            fsPath: "public/beoe", // add this to gitignore
            webPath: "/xs/beoe",
            darkScheme: "class",
          },
        ],
      ],
    }),
  },
});
