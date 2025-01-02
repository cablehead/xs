// @ts-check
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

// https://astro.build/config
export default defineConfig({
  base: "/xs/",
  integrations: [
    starlight({
      title: "xs",

      social: {
        github: "https://github.com/cablehead/xs",
        discord: "https://discord.com/invite/YNbScHBHrh",
      },

      sidebar: [
        {
          label: "Getting Started",
          autogenerate: { directory: "getting-started" },
        },
        {
          label: "Guides",
          autogenerate: { directory: "guides" },
        },
        {
          label: "Reference",
          autogenerate: { directory: "reference" },
        },
      ],

      expressiveCode: {
        themes: ["dracula", "rose-pine-dawn"],
      },
    }),
  ],
});
