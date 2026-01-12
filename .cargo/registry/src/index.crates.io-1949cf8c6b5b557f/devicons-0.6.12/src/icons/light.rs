use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    pub static ref ICONS_MAP: HashMap<&'static str, super::FileIcon> = {
        let mut m = HashMap::new();
        m.insert(
            ".babelrc",
            super::FileIcon {
                icon: '',
                color: "#666620",
            },
        );
        m.insert(
            ".bash_profile",
            super::FileIcon {
                icon: '',
                color: "#447028",
            },
        );
        m.insert(
            ".bashrc",
            super::FileIcon {
                icon: '',
                color: "#447028",
            },
        );
        m.insert(
            ".dockerignore",
            super::FileIcon {
                icon: '󰡨',
                color: "#2e5f99",
            },
        );
        m.insert(
            ".ds_store",
            super::FileIcon {
                icon: '',
                color: "#41535b",
            },
        );
        m.insert(
            ".editorconfig",
            super::FileIcon {
                icon: '',
                color: "#333030",
            },
        );
        m.insert(
            ".env",
            super::FileIcon {
                icon: '',
                color: "#32310d",
            },
        );
        m.insert(
            ".eslintrc",
            super::FileIcon {
                icon: '',
                color: "#4b32c3",
            },
        );
        m.insert(
            ".eslintignore",
            super::FileIcon {
                icon: '',
                color: "#4b32c3",
            },
        );
        m.insert(
            ".git-blame-ignore-revs",
            super::FileIcon {
                icon: '',
                color: "#b83a1d",
            },
        );
        m.insert(
            ".gitattributes",
            super::FileIcon {
                icon: '',
                color: "#b83a1d",
            },
        );
        m.insert(
            ".gitconfig",
            super::FileIcon {
                icon: '',
                color: "#b83a1d",
            },
        );
        m.insert(
            ".gitignore",
            super::FileIcon {
                icon: '',
                color: "#b83a1d",
            },
        );
        m.insert(
            ".gitlab-ci.yml",
            super::FileIcon {
                icon: '',
                color: "#aa321f",
            },
        );
        m.insert(
            ".gitmodules",
            super::FileIcon {
                icon: '',
                color: "#b83a1d",
            },
        );
        m.insert(
            ".gtkrc-2.0",
            super::FileIcon {
                icon: '',
                color: "#333333",
            },
        );
        m.insert(
            ".gvimrc",
            super::FileIcon {
                icon: '',
                color: "#017226",
            },
        );
        m.insert(
            ".justfile",
            super::FileIcon {
                icon: '',
                color: "#526064",
            },
        );
        m.insert(
            ".luaurc",
            super::FileIcon {
                icon: '',
                color: "#007abf",
            },
        );
        m.insert(
            ".mailmap",
            super::FileIcon {
                icon: '󰊢',
                color: "#b83a1d",
            },
        );
        m.insert(
            ".npmignore",
            super::FileIcon {
                icon: '',
                color: "#ae1d38",
            },
        );
        m.insert(
            ".npmrc",
            super::FileIcon {
                icon: '',
                color: "#ae1d38",
            },
        );
        m.insert(
            ".nuxtrc",
            super::FileIcon {
                icon: '󱄆',
                color: "#00835f",
            },
        );
        m.insert(
            ".nvmrc",
            super::FileIcon {
                icon: '',
                color: "#3f6b34",
            },
        );
        m.insert(
            ".prettierrc",
            super::FileIcon {
                icon: '',
                color: "#3264b7",
            },
        );
        m.insert(
            ".prettierrc.json",
            super::FileIcon {
                icon: '',
                color: "#3264b7",
            },
        );
        m.insert(
            ".prettierrc.json5",
            super::FileIcon {
                icon: '',
                color: "#3264b7",
            },
        );
        m.insert(
            ".prettierrc.toml",
            super::FileIcon {
                icon: '',
                color: "#3264b7",
            },
        );
        m.insert(
            ".prettierrc.yaml",
            super::FileIcon {
                icon: '',
                color: "#3264b7",
            },
        );
        m.insert(
            ".prettierrc.yml",
            super::FileIcon {
                icon: '',
                color: "#3264b7",
            },
        );
        m.insert(
            ".prettierignore",
            super::FileIcon {
                icon: '',
                color: "#3264b7",
            },
        );
        m.insert(
            ".settings.json",
            super::FileIcon {
                icon: '',
                color: "#643995",
            },
        );
        m.insert(
            ".SRCINFO",
            super::FileIcon {
                icon: '󰣇',
                color: "#0b6f9e",
            },
        );
        m.insert(
            ".vimrc",
            super::FileIcon {
                icon: '',
                color: "#017226",
            },
        );
        m.insert(
            ".Xauthority",
            super::FileIcon {
                icon: '',
                color: "#ac3a12",
            },
        );
        m.insert(
            ".xinitrc",
            super::FileIcon {
                icon: '',
                color: "#ac3a12",
            },
        );
        m.insert(
            ".Xresources",
            super::FileIcon {
                icon: '',
                color: "#ac3a12",
            },
        );
        m.insert(
            ".xsession",
            super::FileIcon {
                icon: '',
                color: "#ac3a12",
            },
        );
        m.insert(
            ".zprofile",
            super::FileIcon {
                icon: '',
                color: "#447028",
            },
        );
        m.insert(
            ".zshenv",
            super::FileIcon {
                icon: '',
                color: "#447028",
            },
        );
        m.insert(
            ".zshrc",
            super::FileIcon {
                icon: '',
                color: "#447028",
            },
        );
        m.insert(
            "_gvimrc",
            super::FileIcon {
                icon: '',
                color: "#017226",
            },
        );
        m.insert(
            "_vimrc",
            super::FileIcon {
                icon: '',
                color: "#017226",
            },
        );
        m.insert(
            "R",
            super::FileIcon {
                icon: '󰟔',
                color: "#1a4c8c",
            },
        );
        m.insert(
            "avif",
            super::FileIcon {
                icon: '',
                color: "#6b4d83",
            },
        );
        m.insert(
            "brewfile",
            super::FileIcon {
                icon: '',
                color: "#701516",
            },
        );
        m.insert(
            "bspwmrc",
            super::FileIcon {
                icon: '',
                color: "#2f2f2f",
            },
        );
        m.insert(
            "build",
            super::FileIcon {
                icon: '',
                color: "#447028",
            },
        );
        m.insert(
            "build.gradle",
            super::FileIcon {
                icon: '',
                color: "#005f87",
            },
        );
        m.insert(
            "build.zig.zon",
            super::FileIcon {
                icon: '',
                color: "#7b4d0e",
            },
        );
        m.insert(
            "checkhealth",
            super::FileIcon {
                icon: '󰓙',
                color: "#3a5a7e",
            },
        );
        m.insert(
            "cmakelists.txt",
            super::FileIcon {
                icon: '',
                color: "#526064",
            },
        );
        m.insert(
            "code_of_conduct",
            super::FileIcon {
                icon: '',
                color: "#ab104a",
            },
        );
        m.insert(
            "code_of_conduct.md",
            super::FileIcon {
                icon: '',
                color: "#ab104a",
            },
        );
        m.insert(
            "commit_editmsg",
            super::FileIcon {
                icon: '',
                color: "#b83a1d",
            },
        );
        m.insert(
            "commitlint.config.js",
            super::FileIcon {
                icon: '󰜘',
                color: "#207067",
            },
        );
        m.insert(
            "commitlint.config.ts",
            super::FileIcon {
                icon: '󰜘',
                color: "#207067",
            },
        );
        m.insert(
            "compose.yaml",
            super::FileIcon {
                icon: '󰡨',
                color: "#2e5f99",
            },
        );
        m.insert(
            "compose.yml",
            super::FileIcon {
                icon: '󰡨',
                color: "#2e5f99",
            },
        );
        m.insert(
            "config",
            super::FileIcon {
                icon: '',
                color: "#526064",
            },
        );
        m.insert(
            "containerfile",
            super::FileIcon {
                icon: '󰡨',
                color: "#2e5f99",
            },
        );
        m.insert(
            "copying",
            super::FileIcon {
                icon: '',
                color: "#666620",
            },
        );
        m.insert(
            "copying.lesser",
            super::FileIcon {
                icon: '',
                color: "#666620",
            },
        );
        m.insert(
            "docker-compose.yaml",
            super::FileIcon {
                icon: '󰡨',
                color: "#2e5f99",
            },
        );
        m.insert(
            "docker-compose.yml",
            super::FileIcon {
                icon: '󰡨',
                color: "#2e5f99",
            },
        );
        m.insert(
            "dockerfile",
            super::FileIcon {
                icon: '󰡨',
                color: "#2e5f99",
            },
        );
        m.insert(
            "eslint.config.cjs",
            super::FileIcon {
                icon: '',
                color: "#4b32c3",
            },
        );
        m.insert(
            "eslint.config.js",
            super::FileIcon {
                icon: '',
                color: "#4b32c3",
            },
        );
        m.insert(
            "eslint.config.mjs",
            super::FileIcon {
                icon: '',
                color: "#4b32c3",
            },
        );
        m.insert(
            "eslint.config.ts",
            super::FileIcon {
                icon: '',
                color: "#4b32c3",
            },
        );
        m.insert(
            "ext_typoscript_setup.txt",
            super::FileIcon {
                icon: '',
                color: "#aa5a00",
            },
        );
        m.insert(
            "favicon.ico",
            super::FileIcon {
                icon: '',
                color: "#666620",
            },
        );
        m.insert(
            "fp-info-cache",
            super::FileIcon {
                icon: '',
                color: "#333333",
            },
        );
        m.insert(
            "fp-lib-table",
            super::FileIcon {
                icon: '',
                color: "#333333",
            },
        );
        m.insert(
            "FreeCAD.conf",
            super::FileIcon {
                icon: '',
                color: "#98262c",
            },
        );
        m.insert(
            "gemfile$",
            super::FileIcon {
                icon: '',
                color: "#701516",
            },
        );
        m.insert(
            "gnumakefile",
            super::FileIcon {
                icon: '',
                color: "#526064",
            },
        );
        m.insert(
            "go.mod",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "go.sum",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "go.work",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "gradlew",
            super::FileIcon {
                icon: '',
                color: "#005f87",
            },
        );
        m.insert(
            "gradle.properties",
            super::FileIcon {
                icon: '',
                color: "#005f87",
            },
        );
        m.insert(
            "gradle-wrapper.properties",
            super::FileIcon {
                icon: '',
                color: "#005f87",
            },
        );
        m.insert(
            "groovy",
            super::FileIcon {
                icon: '',
                color: "#384e5d",
            },
        );
        m.insert(
            "gruntfile.babel.js",
            super::FileIcon {
                icon: '',
                color: "#975122",
            },
        );
        m.insert(
            "gruntfile.coffee",
            super::FileIcon {
                icon: '',
                color: "#975122",
            },
        );
        m.insert(
            "gruntfile.js",
            super::FileIcon {
                icon: '',
                color: "#975122",
            },
        );
        m.insert(
            "gruntfile.ts",
            super::FileIcon {
                icon: '',
                color: "#975122",
            },
        );
        m.insert(
            "gtkrc",
            super::FileIcon {
                icon: '',
                color: "#333333",
            },
        );
        m.insert(
            "gulpfile.babel.js",
            super::FileIcon {
                icon: '',
                color: "#992e33",
            },
        );
        m.insert(
            "gulpfile.coffee",
            super::FileIcon {
                icon: '',
                color: "#992e33",
            },
        );
        m.insert(
            "gulpfile.js",
            super::FileIcon {
                icon: '',
                color: "#992e33",
            },
        );
        m.insert(
            "gulpfile.ts",
            super::FileIcon {
                icon: '',
                color: "#992e33",
            },
        );
        m.insert(
            "hypridle.conf",
            super::FileIcon {
                icon: '',
                color: "#008082",
            },
        );
        m.insert(
            "hyprland.conf",
            super::FileIcon {
                icon: '',
                color: "#008082",
            },
        );
        m.insert(
            "hyprlock.conf",
            super::FileIcon {
                icon: '',
                color: "#008082",
            },
        );
        m.insert(
            "i18n.config.js",
            super::FileIcon {
                icon: '󰗊',
                color: "#515987",
            },
        );
        m.insert(
            "i18n.config.ts",
            super::FileIcon {
                icon: '󰗊',
                color: "#515987",
            },
        );
        m.insert(
            "i3blocks.conf",
            super::FileIcon {
                icon: '',
                color: "#2e2f30",
            },
        );
        m.insert(
            "i3status.conf",
            super::FileIcon {
                icon: '',
                color: "#2e2f30",
            },
        );
        m.insert(
            "ionic.config.json",
            super::FileIcon {
                icon: '',
                color: "#355fa5",
            },
        );
        m.insert(
            "cantorrc",
            super::FileIcon {
                icon: '',
                color: "#1573b6",
            },
        );
        m.insert(
            "justfile",
            super::FileIcon {
                icon: '',
                color: "#526064",
            },
        );
        m.insert(
            "kalgebrarc",
            super::FileIcon {
                icon: '',
                color: "#1573b6",
            },
        );
        m.insert(
            "kdeglobals",
            super::FileIcon {
                icon: '',
                color: "#1573b6",
            },
        );
        m.insert(
            "kdenlive-layoutsrc",
            super::FileIcon {
                icon: '',
                color: "#425c79",
            },
        );
        m.insert(
            "kdenliverc",
            super::FileIcon {
                icon: '',
                color: "#425c79",
            },
        );
        m.insert(
            "kritadisplayrc",
            super::FileIcon {
                icon: '',
                color: "#a12ea7",
            },
        );
        m.insert(
            "kritarc",
            super::FileIcon {
                icon: '',
                color: "#a12ea7",
            },
        );
        m.insert(
            "license",
            super::FileIcon {
                icon: '',
                color: "#686020",
            },
        );
        m.insert(
            "license.md",
            super::FileIcon {
                icon: '',
                color: "#686020",
            },
        );
        m.insert(
            "lxde-rc.xml",
            super::FileIcon {
                icon: '',
                color: "#606060",
            },
        );
        m.insert(
            "lxqt.conf",
            super::FileIcon {
                icon: '',
                color: "#016e9e",
            },
        );
        m.insert(
            "makefile",
            super::FileIcon {
                icon: '',
                color: "#526064",
            },
        );
        m.insert(
            "mix.lock",
            super::FileIcon {
                icon: '',
                color: "#6b4d83",
            },
        );
        m.insert(
            "mpv.conf",
            super::FileIcon {
                icon: '',
                color: "#3b1342",
            },
        );
        m.insert(
            "node_modules",
            super::FileIcon {
                icon: '',
                color: "#ae1d38",
            },
        );
        m.insert(
            "nuxt.config.cjs",
            super::FileIcon {
                icon: '󱄆',
                color: "#00835f",
            },
        );
        m.insert(
            "nuxt.config.js",
            super::FileIcon {
                icon: '󱄆',
                color: "#00835f",
            },
        );
        m.insert(
            "nuxt.config.mjs",
            super::FileIcon {
                icon: '󱄆',
                color: "#00835f",
            },
        );
        m.insert(
            "nuxt.config.ts",
            super::FileIcon {
                icon: '󱄆',
                color: "#00835f",
            },
        );
        m.insert(
            "package.json",
            super::FileIcon {
                icon: '',
                color: "#ae1d38",
            },
        );
        m.insert(
            "package-lock.json",
            super::FileIcon {
                icon: '',
                color: "#7a0d21",
            },
        );
        m.insert(
            "PKGBUILD",
            super::FileIcon {
                icon: '',
                color: "#0b6f9e",
            },
        );
        m.insert(
            "platformio.ini",
            super::FileIcon {
                icon: '',
                color: "#a4571d",
            },
        );
        m.insert(
            "pom.xml",
            super::FileIcon {
                icon: '',
                color: "#7a0d21",
            },
        );
        m.insert(
            "prettier.config.js",
            super::FileIcon {
                icon: '',
                color: "#3264b7",
            },
        );
        m.insert(
            "prettier.config.cjs",
            super::FileIcon {
                icon: '',
                color: "#3264b7",
            },
        );
        m.insert(
            "prettier.config.mjs",
            super::FileIcon {
                icon: '',
                color: "#3264b7",
            },
        );
        m.insert(
            "prettier.config.ts",
            super::FileIcon {
                icon: '',
                color: "#3264b7",
            },
        );
        m.insert(
            "procfile",
            super::FileIcon {
                icon: '',
                color: "#6b4d83",
            },
        );
        m.insert(
            "PrusaSlicer.ini",
            super::FileIcon {
                icon: '',
                color: "#9d4717",
            },
        );
        m.insert(
            "PrusaSlicerGcodeViewer.ini",
            super::FileIcon {
                icon: '',
                color: "#9d4717",
            },
        );
        m.insert(
            "py.typed",
            super::FileIcon {
                icon: '',
                color: "#805e02",
            },
        );
        m.insert(
            "QtProject.conf",
            super::FileIcon {
                icon: '',
                color: "#2b8937",
            },
        );
        m.insert(
            "r",
            super::FileIcon {
                icon: '󰟔',
                color: "#1a4c8c",
            },
        );
        m.insert(
            "rakefile",
            super::FileIcon {
                icon: '',
                color: "#701516",
            },
        );
        m.insert(
            "rmd",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "robots.txt",
            super::FileIcon {
                icon: '󰚩',
                color: "#465470",
            },
        );
        m.insert(
            "security",
            super::FileIcon {
                icon: '󰒃',
                color: "#3f4143",
            },
        );
        m.insert(
            "security.md",
            super::FileIcon {
                icon: '󰒃',
                color: "#3f4143",
            },
        );
        m.insert(
            "settings.gradle",
            super::FileIcon {
                icon: '',
                color: "#005f87",
            },
        );
        m.insert(
            "svelte.config.js",
            super::FileIcon {
                icon: '',
                color: "#bf2e00",
            },
        );
        m.insert(
            "sxhkdrc",
            super::FileIcon {
                icon: '',
                color: "#2f2f2f",
            },
        );
        m.insert(
            "sym-lib-table",
            super::FileIcon {
                icon: '',
                color: "#333333",
            },
        );
        m.insert(
            "tailwind.config.js",
            super::FileIcon {
                icon: '󱏿',
                color: "#158197",
            },
        );
        m.insert(
            "tailwind.config.mjs",
            super::FileIcon {
                icon: '󱏿',
                color: "#158197",
            },
        );
        m.insert(
            "tailwind.config.ts",
            super::FileIcon {
                icon: '󱏿',
                color: "#158197",
            },
        );
        m.insert(
            "tmux.conf",
            super::FileIcon {
                icon: '',
                color: "#0f8c13",
            },
        );
        m.insert(
            "tmux.conf.local",
            super::FileIcon {
                icon: '',
                color: "#0f8c13",
            },
        );
        m.insert(
            "tsconfig.json",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "unlicense",
            super::FileIcon {
                icon: '',
                color: "#686020",
            },
        );
        m.insert(
            "vagrantfile$",
            super::FileIcon {
                icon: '',
                color: "#104abf",
            },
        );
        m.insert(
            "vlcrc",
            super::FileIcon {
                icon: '󰕼',
                color: "#9f5100",
            },
        );
        m.insert(
            "vercel.json",
            super::FileIcon {
                icon: '▲',
                color: "#333333",
            },
        );
        m.insert(
            "webpack",
            super::FileIcon {
                icon: '󰜫',
                color: "#36677c",
            },
        );
        m.insert(
            "weston.ini",
            super::FileIcon {
                icon: '',
                color: "#805e00",
            },
        );
        m.insert(
            "workspace",
            super::FileIcon {
                icon: '',
                color: "#447028",
            },
        );
        m.insert(
            "xmobarrc",
            super::FileIcon {
                icon: '',
                color: "#a9333e",
            },
        );
        m.insert(
            "xmobarrc.hs",
            super::FileIcon {
                icon: '',
                color: "#a9333e",
            },
        );
        m.insert(
            "xmonad.hs",
            super::FileIcon {
                icon: '',
                color: "#a9333e",
            },
        );
        m.insert(
            "xorg.conf",
            super::FileIcon {
                icon: '',
                color: "#ac3a12",
            },
        );
        m.insert(
            "xsettingsd.conf",
            super::FileIcon {
                icon: '',
                color: "#ac3a12",
            },
        );
        m.insert(
            "3gp",
            super::FileIcon {
                icon: '',
                color: "#7e4c10",
            },
        );
        m.insert(
            "3mf",
            super::FileIcon {
                icon: '󰆧',
                color: "#5b5b5b",
            },
        );
        m.insert(
            "7z",
            super::FileIcon {
                icon: '',
                color: "#76520c",
            },
        );
        m.insert(
            "a",
            super::FileIcon {
                icon: '',
                color: "#494a47",
            },
        );
        m.insert(
            "aac",
            super::FileIcon {
                icon: '',
                color: "#0075aa",
            },
        );
        m.insert(
            "aif",
            super::FileIcon {
                icon: '',
                color: "#0075aa",
            },
        );
        m.insert(
            "aiff",
            super::FileIcon {
                icon: '',
                color: "#0075aa",
            },
        );
        m.insert(
            "ape",
            super::FileIcon {
                icon: '',
                color: "#0075aa",
            },
        );
        m.insert(
            "apl",
            super::FileIcon {
                icon: '⍝',
                color: "#805200",
            },
        );
        m.insert(
            "ai",
            super::FileIcon {
                icon: '',
                color: "#666620",
            },
        );
        m.insert(
            "android",
            super::FileIcon {
                icon: '',
                color: "#277e3e",
            },
        );
        m.insert(
            "apk",
            super::FileIcon {
                icon: '',
                color: "#277e3e",
            },
        );
        m.insert(
            "app",
            super::FileIcon {
                icon: '',
                color: "#9F0500",
            },
        );
        m.insert(
            "applescript",
            super::FileIcon {
                icon: '',
                color: "#526064",
            },
        );
        m.insert(
            "asc",
            super::FileIcon {
                icon: '󰦝',
                color: "#41525f",
            },
        );
        m.insert(
            "ass",
            super::FileIcon {
                icon: '󰨖',
                color: "#805c0a",
            },
        );
        m.insert(
            "astro",
            super::FileIcon {
                icon: '',
                color: "#aa2f4d",
            },
        );
        m.insert(
            "awk",
            super::FileIcon {
                icon: '',
                color: "#3a4446",
            },
        );
        m.insert(
            "azcli",
            super::FileIcon {
                icon: '',
                color: "#005a9f",
            },
        );
        m.insert(
            "bak",
            super::FileIcon {
                icon: '󰁯',
                color: "#526064",
            },
        );
        m.insert(
            "bash",
            super::FileIcon {
                icon: '',
                color: "#447028",
            },
        );
        m.insert(
            "bat",
            super::FileIcon {
                icon: '',
                color: "#40500f",
            },
        );
        m.insert(
            "bazel",
            super::FileIcon {
                icon: '',
                color: "#447028",
            },
        );
        m.insert(
            "bib",
            super::FileIcon {
                icon: '󱉟',
                color: "#666620",
            },
        );
        m.insert(
            "bicep",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "bicepparam",
            super::FileIcon {
                icon: '',
                color: "#6a4d77",
            },
        );
        m.insert(
            "bin",
            super::FileIcon {
                icon: '',
                color: "#9F0500",
            },
        );
        m.insert(
            "blade.php",
            super::FileIcon {
                icon: '',
                color: "#a0372b",
            },
        );
        m.insert(
            "blend",
            super::FileIcon {
                icon: '󰂫',
                color: "#9c4f00",
            },
        );
        m.insert(
            "bmp",
            super::FileIcon {
                icon: '',
                color: "#6b4d83",
            },
        );
        m.insert(
            "bqn",
            super::FileIcon {
                icon: '⎉',
                color: "#20544d",
            },
        );
        m.insert(
            "blp",
            super::FileIcon {
                icon: '󰺾',
                color: "#3a6497",
            },
        );
        m.insert(
            "brep",
            super::FileIcon {
                icon: '󰻫',
                color: "#576342",
            },
        );
        m.insert(
            "bz",
            super::FileIcon {
                icon: '',
                color: "#76520c",
            },
        );
        m.insert(
            "bz2",
            super::FileIcon {
                icon: '',
                color: "#76520c",
            },
        );
        m.insert(
            "bz3",
            super::FileIcon {
                icon: '',
                color: "#76520c",
            },
        );
        m.insert(
            "bzl",
            super::FileIcon {
                icon: '',
                color: "#447028",
            },
        );
        m.insert(
            "c",
            super::FileIcon {
                icon: '',
                color: "#3b69aa",
            },
        );
        m.insert(
            "c++",
            super::FileIcon {
                icon: '',
                color: "#a23253",
            },
        );
        m.insert(
            "cache",
            super::FileIcon {
                icon: '',
                color: "#333333",
            },
        );
        m.insert(
            "cast",
            super::FileIcon {
                icon: '',
                color: "#7e4c10",
            },
        );
        m.insert(
            "cbl",
            super::FileIcon {
                icon: '⚙',
                color: "#005ca5",
            },
        );
        m.insert(
            "cc",
            super::FileIcon {
                icon: '',
                color: "#a23253",
            },
        );
        m.insert(
            "ccm",
            super::FileIcon {
                icon: '',
                color: "#a23253",
            },
        );
        m.insert(
            "cfg",
            super::FileIcon {
                icon: '',
                color: "#526064",
            },
        );
        m.insert(
            "cjs",
            super::FileIcon {
                icon: '',
                color: "#666620",
            },
        );
        m.insert(
            "clj",
            super::FileIcon {
                icon: '',
                color: "#466024",
            },
        );
        m.insert(
            "cljc",
            super::FileIcon {
                icon: '',
                color: "#466024",
            },
        );
        m.insert(
            "cljs",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "cljd",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "cmake",
            super::FileIcon {
                icon: '',
                color: "#526064",
            },
        );
        m.insert(
            "cob",
            super::FileIcon {
                icon: '⚙',
                color: "#005ca5",
            },
        );
        m.insert(
            "cobol",
            super::FileIcon {
                icon: '⚙',
                color: "#005ca5",
            },
        );
        m.insert(
            "coffee",
            super::FileIcon {
                icon: '',
                color: "#666620",
            },
        );
        m.insert(
            "conf",
            super::FileIcon {
                icon: '',
                color: "#526064",
            },
        );
        m.insert(
            "config.ru",
            super::FileIcon {
                icon: '',
                color: "#701516",
            },
        );
        m.insert(
            "cow",
            super::FileIcon {
                icon: '󰆚',
                color: "#70421b",
            },
        );
        m.insert(
            "cp",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "cpp",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "cppm",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "cpy",
            super::FileIcon {
                icon: '⚙',
                color: "#005ca5",
            },
        );
        m.insert(
            "cr",
            super::FileIcon {
                icon: '',
                color: "#434343",
            },
        );
        m.insert(
            "crdownload",
            super::FileIcon {
                icon: '',
                color: "#226654",
            },
        );
        m.insert(
            "cs",
            super::FileIcon {
                icon: '󰌛',
                color: "#434d04",
            },
        );
        m.insert(
            "csh",
            super::FileIcon {
                icon: '',
                color: "#3a4446",
            },
        );
        m.insert(
            "cshtml",
            super::FileIcon {
                icon: '󱦗',
                color: "#512bd4",
            },
        );
        m.insert(
            "cson",
            super::FileIcon {
                icon: '',
                color: "#666620",
            },
        );
        m.insert(
            "csproj",
            super::FileIcon {
                icon: '󰪮',
                color: "#512bd4",
            },
        );
        m.insert(
            "css",
            super::FileIcon {
                icon: '',
                color: "#2c6ea3",
            },
        );
        m.insert(
            "csv",
            super::FileIcon {
                icon: '',
                color: "#447028",
            },
        );
        m.insert(
            "cts",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "cu",
            super::FileIcon {
                icon: '',
                color: "#447028",
            },
        );
        m.insert(
            "cue",
            super::FileIcon {
                icon: '󰲹',
                color: "#764a57",
            },
        );
        m.insert(
            "cuh",
            super::FileIcon {
                icon: '',
                color: "#6b4d83",
            },
        );
        m.insert(
            "cxx",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "cxxm",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "d",
            super::FileIcon {
                icon: '',
                color: "#325a13",
            },
        );
        m.insert(
            "d.ts",
            super::FileIcon {
                icon: '',
                color: "#6a4c2a",
            },
        );
        m.insert(
            "dart",
            super::FileIcon {
                icon: '',
                color: "#03589C",
            },
        );
        m.insert(
            "db",
            super::FileIcon {
                icon: '',
                color: "#494848",
            },
        );
        m.insert(
            "dconf",
            super::FileIcon {
                icon: '',
                color: "#333333",
            },
        );
        m.insert(
            "desktop",
            super::FileIcon {
                icon: '',
                color: "#563d7c",
            },
        );
        m.insert(
            "diff",
            super::FileIcon {
                icon: '',
                color: "#41535b",
            },
        );
        m.insert(
            "dll",
            super::FileIcon {
                icon: '',
                color: "#4d2c0b",
            },
        );
        m.insert(
            "doc",
            super::FileIcon {
                icon: '󰈬',
                color: "#185abd",
            },
        );
        m.insert(
            "Dockerfile",
            super::FileIcon {
                icon: '󰡨',
                color: "#2e5f99",
            },
        );
        m.insert(
            "docx",
            super::FileIcon {
                icon: '󰈬',
                color: "#185abd",
            },
        );
        m.insert(
            "dot",
            super::FileIcon {
                icon: '󱁉',
                color: "#244a6a",
            },
        );
        m.insert(
            "download",
            super::FileIcon {
                icon: '',
                color: "#226654",
            },
        );
        m.insert(
            "drl",
            super::FileIcon {
                icon: '',
                color: "#553a3a",
            },
        );
        m.insert(
            "dropbox",
            super::FileIcon {
                icon: '',
                color: "#0049be",
            },
        );
        m.insert(
            "dump",
            super::FileIcon {
                icon: '',
                color: "#494848",
            },
        );
        m.insert(
            "dwg",
            super::FileIcon {
                icon: '󰻫',
                color: "#576342",
            },
        );
        m.insert(
            "dxf",
            super::FileIcon {
                icon: '󰻫',
                color: "#576342",
            },
        );
        m.insert(
            "ebook",
            super::FileIcon {
                icon: '',
                color: "#755836",
            },
        );
        m.insert(
            "edn",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "eex",
            super::FileIcon {
                icon: '',
                color: "#6b4d83",
            },
        );
        m.insert(
            "ejs",
            super::FileIcon {
                icon: '',
                color: "#666620",
            },
        );
        m.insert(
            "elf",
            super::FileIcon {
                icon: '',
                color: "#9F0500",
            },
        );
        m.insert(
            "el",
            super::FileIcon {
                icon: '',
                color: "#61568e",
            },
        );
        m.insert(
            "elc",
            super::FileIcon {
                icon: '',
                color: "#61568e",
            },
        );
        m.insert(
            "elm",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "eln",
            super::FileIcon {
                icon: '',
                color: "#61568e",
            },
        );
        m.insert(
            "env",
            super::FileIcon {
                icon: '',
                color: "#32310d",
            },
        );
        m.insert(
            "eot",
            super::FileIcon {
                icon: '',
                color: "#2f2f2f",
            },
        );
        m.insert(
            "epp",
            super::FileIcon {
                icon: '',
                color: "#80530d",
            },
        );
        m.insert(
            "epub",
            super::FileIcon {
                icon: '',
                color: "#755836",
            },
        );
        m.insert(
            "erb",
            super::FileIcon {
                icon: '',
                color: "#701516",
            },
        );
        m.insert(
            "erl",
            super::FileIcon {
                icon: '',
                color: "#8a2b72",
            },
        );
        m.insert(
            "ex",
            super::FileIcon {
                icon: '',
                color: "#6b4d83",
            },
        );
        m.insert(
            "exe",
            super::FileIcon {
                icon: '',
                color: "#9F0500",
            },
        );
        m.insert(
            "exs",
            super::FileIcon {
                icon: '',
                color: "#6b4d83",
            },
        );
        m.insert(
            "f#",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "f3d",
            super::FileIcon {
                icon: '󰻫',
                color: "#576342",
            },
        );
        m.insert(
            "f90",
            super::FileIcon {
                icon: '󱈚',
                color: "#563b70",
            },
        );
        m.insert(
            "fbx",
            super::FileIcon {
                icon: '󰆧',
                color: "#5b5b5b",
            },
        );
        m.insert(
            "fcbak",
            super::FileIcon {
                icon: '',
                color: "#98262c",
            },
        );
        m.insert(
            "fcmacro",
            super::FileIcon {
                icon: '',
                color: "#98262c",
            },
        );
        m.insert(
            "fcmat",
            super::FileIcon {
                icon: '',
                color: "#98262c",
            },
        );
        m.insert(
            "fcparam",
            super::FileIcon {
                icon: '',
                color: "#98262c",
            },
        );
        m.insert(
            "fcscript",
            super::FileIcon {
                icon: '',
                color: "#98262c",
            },
        );
        m.insert(
            "fcstd",
            super::FileIcon {
                icon: '',
                color: "#98262c",
            },
        );
        m.insert(
            "fcstd1",
            super::FileIcon {
                icon: '',
                color: "#98262c",
            },
        );
        m.insert(
            "fctb",
            super::FileIcon {
                icon: '',
                color: "#98262c",
            },
        );
        m.insert(
            "fctl",
            super::FileIcon {
                icon: '',
                color: "#98262c",
            },
        );
        m.insert(
            "fdmdownload",
            super::FileIcon {
                icon: '',
                color: "#226654",
            },
        );
        m.insert(
            "flac",
            super::FileIcon {
                icon: '',
                color: "#005880",
            },
        );
        m.insert(
            "flc",
            super::FileIcon {
                icon: '',
                color: "#2f2f2f",
            },
        );
        m.insert(
            "flf",
            super::FileIcon {
                icon: '',
                color: "#2f2f2f",
            },
        );
        m.insert(
            "fnl",
            super::FileIcon {
                icon: '',
                color: "#33312b",
            },
        );
        m.insert(
            "fish",
            super::FileIcon {
                icon: '',
                color: "#3a4446",
            },
        );
        m.insert(
            "fs",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "fsi",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "fsscript",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "fsx",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "gcode",
            super::FileIcon {
                icon: '󰐫',
                color: "#0f5582",
            },
        );
        m.insert(
            "gd",
            super::FileIcon {
                icon: '',
                color: "#526064",
            },
        );
        m.insert(
            "gemspec",
            super::FileIcon {
                icon: '',
                color: "#701516",
            },
        );
        m.insert(
            "gif",
            super::FileIcon {
                icon: '',
                color: "#6b4d83",
            },
        );
        m.insert(
            "git",
            super::FileIcon {
                icon: '',
                color: "#b5391e",
            },
        );
        m.insert(
            "glb",
            super::FileIcon {
                icon: '',
                color: "#80581e",
            },
        );
        m.insert(
            "gleam",
            super::FileIcon {
                icon: '',
                color: "#553a51",
            },
        );
        m.insert(
            "gnumakefile",
            super::FileIcon {
                icon: '',
                color: "#526064",
            },
        );
        m.insert(
            "go",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "godot",
            super::FileIcon {
                icon: '',
                color: "#526064",
            },
        );
        m.insert(
            "gql",
            super::FileIcon {
                icon: '',
                color: "#ac2880",
            },
        );
        m.insert(
            "gradle",
            super::FileIcon {
                icon: '',
                color: "#005f87",
            },
        );
        m.insert(
            "graphql",
            super::FileIcon {
                icon: '',
                color: "#ac2880",
            },
        );
        m.insert(
            "gresource",
            super::FileIcon {
                icon: '',
                color: "#333333",
            },
        );
        m.insert(
            "gv",
            super::FileIcon {
                icon: '󱁉',
                color: "#244a6a",
            },
        );
        m.insert(
            "gz",
            super::FileIcon {
                icon: '',
                color: "#76520c",
            },
        );
        m.insert(
            "h",
            super::FileIcon {
                icon: '',
                color: "#6b4d83",
            },
        );
        m.insert(
            "haml",
            super::FileIcon {
                icon: '',
                color: "#2f2f2d",
            },
        );
        m.insert(
            "hx",
            super::FileIcon {
                icon: '',
                color: "#9c5715",
            },
        );
        m.insert(
            "hbs",
            super::FileIcon {
                icon: '',
                color: "#a04f1d",
            },
        );
        m.insert(
            "hex",
            super::FileIcon {
                icon: '',
                color: "#224abf",
            },
        );
        m.insert(
            "heex",
            super::FileIcon {
                icon: '',
                color: "#6b4d83",
            },
        );
        m.insert(
            "hh",
            super::FileIcon {
                icon: '',
                color: "#6b4d83",
            },
        );
        m.insert(
            "hpp",
            super::FileIcon {
                icon: '',
                color: "#6b4d83",
            },
        );
        m.insert(
            "hrl",
            super::FileIcon {
                icon: '',
                color: "#8a2b72",
            },
        );
        m.insert(
            "hs",
            super::FileIcon {
                icon: '',
                color: "#6b4d83",
            },
        );
        m.insert(
            "htm",
            super::FileIcon {
                icon: '',
                color: "#aa391c",
            },
        );
        m.insert(
            "html",
            super::FileIcon {
                icon: '',
                color: "#ab3a1c",
            },
        );
        m.insert(
            "huff",
            super::FileIcon {
                icon: '󰡘',
                color: "#4242c7",
            },
        );
        m.insert(
            "hurl",
            super::FileIcon {
                icon: '',
                color: "#bf0266",
            },
        );
        m.insert(
            "hxx",
            super::FileIcon {
                icon: '',
                color: "#6b4d83",
            },
        );
        m.insert(
            "ixx",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "ico",
            super::FileIcon {
                icon: '',
                color: "#666620",
            },
        );
        m.insert(
            "ical",
            super::FileIcon {
                icon: '',
                color: "#2B2e83",
            },
        );
        m.insert(
            "icalendar",
            super::FileIcon {
                icon: '',
                color: "#2B2e83",
            },
        );
        m.insert(
            "ics",
            super::FileIcon {
                icon: '',
                color: "#2B2e83",
            },
        );
        m.insert(
            "ifb",
            super::FileIcon {
                icon: '',
                color: "#2B2e83",
            },
        );
        m.insert(
            "ifc",
            super::FileIcon {
                icon: '󰻫',
                color: "#576342",
            },
        );
        m.insert(
            "ige",
            super::FileIcon {
                icon: '󰻫',
                color: "#576342",
            },
        );
        m.insert(
            "iges",
            super::FileIcon {
                icon: '󰻫',
                color: "#576342",
            },
        );
        m.insert(
            "igs",
            super::FileIcon {
                icon: '󰻫',
                color: "#576342",
            },
        );
        m.insert(
            "image",
            super::FileIcon {
                icon: '',
                color: "#453f43",
            },
        );
        m.insert(
            "img",
            super::FileIcon {
                icon: '',
                color: "#453f43",
            },
        );
        m.insert(
            "import",
            super::FileIcon {
                icon: '',
                color: "#2f2f2f",
            },
        );
        m.insert(
            "info",
            super::FileIcon {
                icon: '',
                color: "#333329",
            },
        );
        m.insert(
            "ini",
            super::FileIcon {
                icon: '',
                color: "#526064",
            },
        );
        m.insert(
            "ino",
            super::FileIcon {
                icon: '',
                color: "#397981",
            },
        );
        m.insert(
            "iso",
            super::FileIcon {
                icon: '',
                color: "#453f43",
            },
        );
        m.insert(
            "ipynb",
            super::FileIcon {
                icon: '',
                color: "#366b8a",
            },
        );
        m.insert(
            "java",
            super::FileIcon {
                icon: '',
                color: "#992e33",
            },
        );
        m.insert(
            "jl",
            super::FileIcon {
                icon: '',
                color: "#6c4b7c",
            },
        );
        m.insert(
            "jwmrc",
            super::FileIcon {
                icon: '',
                color: "#005a9a",
            },
        );
        m.insert(
            "jpeg",
            super::FileIcon {
                icon: '',
                color: "#6b4d83",
            },
        );
        m.insert(
            "jpg",
            super::FileIcon {
                icon: '',
                color: "#6b4d83",
            },
        );
        m.insert(
            "js",
            super::FileIcon {
                icon: '',
                color: "#666620",
            },
        );
        m.insert(
            "json",
            super::FileIcon {
                icon: '',
                color: "#666620",
            },
        );
        m.insert(
            "json5",
            super::FileIcon {
                icon: '',
                color: "#666620",
            },
        );
        m.insert(
            "jsonc",
            super::FileIcon {
                icon: '',
                color: "#666620",
            },
        );
        m.insert(
            "jsx",
            super::FileIcon {
                icon: '',
                color: "#158197",
            },
        );
        m.insert(
            "jxl",
            super::FileIcon {
                icon: '',
                color: "#6b4d83",
            },
        );
        m.insert(
            "kbx",
            super::FileIcon {
                icon: '󰯄',
                color: "#565856",
            },
        );
        m.insert(
            "kdb",
            super::FileIcon {
                icon: '',
                color: "#3e7427",
            },
        );
        m.insert(
            "kdbx",
            super::FileIcon {
                icon: '',
                color: "#3e7427",
            },
        );
        m.insert(
            "kdenlive",
            super::FileIcon {
                icon: '',
                color: "#425c79",
            },
        );
        m.insert(
            "kdenlivetitle",
            super::FileIcon {
                icon: '',
                color: "#425c79",
            },
        );
        m.insert(
            "kicad_dru",
            super::FileIcon {
                icon: '',
                color: "#333333",
            },
        );
        m.insert(
            "kicad_mod",
            super::FileIcon {
                icon: '',
                color: "#333333",
            },
        );
        m.insert(
            "kicad_pcb",
            super::FileIcon {
                icon: '',
                color: "#333333",
            },
        );
        m.insert(
            "kicad_prl",
            super::FileIcon {
                icon: '',
                color: "#333333",
            },
        );
        m.insert(
            "kicad_pro",
            super::FileIcon {
                icon: '',
                color: "#333333",
            },
        );
        m.insert(
            "kicad_sch",
            super::FileIcon {
                icon: '',
                color: "#333333",
            },
        );
        m.insert(
            "kicad_sym",
            super::FileIcon {
                icon: '',
                color: "#333333",
            },
        );
        m.insert(
            "kicad_wks",
            super::FileIcon {
                icon: '',
                color: "#333333",
            },
        );
        m.insert(
            "ko",
            super::FileIcon {
                icon: '',
                color: "#494a47",
            },
        );
        m.insert(
            "kpp",
            super::FileIcon {
                icon: '',
                color: "#a12ea7",
            },
        );
        m.insert(
            "kra",
            super::FileIcon {
                icon: '',
                color: "#a12ea7",
            },
        );
        m.insert(
            "krz",
            super::FileIcon {
                icon: '',
                color: "#a12ea7",
            },
        );
        m.insert(
            "ksh",
            super::FileIcon {
                icon: '',
                color: "#3a4446",
            },
        );
        m.insert(
            "kt",
            super::FileIcon {
                icon: '',
                color: "#5f3ebf",
            },
        );
        m.insert(
            "kts",
            super::FileIcon {
                icon: '',
                color: "#5f3ebf",
            },
        );
        m.insert(
            "lck",
            super::FileIcon {
                icon: '',
                color: "#5e5e5e",
            },
        );
        m.insert(
            "leex",
            super::FileIcon {
                icon: '',
                color: "#6b4d83",
            },
        );
        m.insert(
            "less",
            super::FileIcon {
                icon: '',
                color: "#563d7c",
            },
        );
        m.insert(
            "lff",
            super::FileIcon {
                icon: '',
                color: "#2f2f2f",
            },
        );
        m.insert(
            "lhs",
            super::FileIcon {
                icon: '',
                color: "#6b4d83",
            },
        );
        m.insert(
            "lib",
            super::FileIcon {
                icon: '',
                color: "#4d2c0b",
            },
        );
        m.insert(
            "license",
            super::FileIcon {
                icon: '',
                color: "#666620",
            },
        );
        m.insert(
            "liquid",
            super::FileIcon {
                icon: '',
                color: "#4a6024",
            },
        );
        m.insert(
            "lock",
            super::FileIcon {
                icon: '',
                color: "#5e5e5e",
            },
        );
        m.insert(
            "log",
            super::FileIcon {
                icon: '󰌱',
                color: "#4a4a4a",
            },
        );
        m.insert(
            "lrc",
            super::FileIcon {
                icon: '󰨖',
                color: "#805c0a",
            },
        );
        m.insert(
            "lua",
            super::FileIcon {
                icon: '',
                color: "#366b8a",
            },
        );
        m.insert(
            "luac",
            super::FileIcon {
                icon: '',
                color: "#366b8a",
            },
        );
        m.insert(
            "luau",
            super::FileIcon {
                icon: '',
                color: "#007abf",
            },
        );
        m.insert(
            "m3u",
            super::FileIcon {
                icon: '󰲹',
                color: "#764a57",
            },
        );
        m.insert(
            "m3u8",
            super::FileIcon {
                icon: '󰲹',
                color: "#764a57",
            },
        );
        m.insert(
            "m4a",
            super::FileIcon {
                icon: '',
                color: "#0075aa",
            },
        );
        m.insert(
            "m4v",
            super::FileIcon {
                icon: '',
                color: "#7e4c10",
            },
        );
        m.insert(
            "magnet",
            super::FileIcon {
                icon: '',
                color: "#a51b16",
            },
        );
        m.insert(
            "makefile",
            super::FileIcon {
                icon: '',
                color: "#526064",
            },
        );
        m.insert(
            "markdown",
            super::FileIcon {
                icon: '',
                color: "#4a4a4a",
            },
        );
        m.insert(
            "material",
            super::FileIcon {
                icon: '󰔉',
                color: "#8a2b72",
            },
        );
        m.insert(
            "md",
            super::FileIcon {
                icon: '',
                color: "#4a4a4a",
            },
        );
        m.insert(
            "md5",
            super::FileIcon {
                icon: '󰕥',
                color: "#5d5975",
            },
        );
        m.insert(
            "mdx",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "mint",
            super::FileIcon {
                icon: '󰌪',
                color: "#44604a",
            },
        );
        m.insert(
            "mjs",
            super::FileIcon {
                icon: '',
                color: "#504b1e",
            },
        );
        m.insert(
            "mk",
            super::FileIcon {
                icon: '',
                color: "#526064",
            },
        );
        m.insert(
            "mkv",
            super::FileIcon {
                icon: '',
                color: "#7e4c10",
            },
        );
        m.insert(
            "ml",
            super::FileIcon {
                icon: '',
                color: "#975122",
            },
        );
        m.insert(
            "mli",
            super::FileIcon {
                icon: '',
                color: "#975122",
            },
        );
        m.insert(
            "m",
            super::FileIcon {
                icon: '',
                color: "#3b69aa",
            },
        );
        m.insert(
            "mm",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "mo",
            super::FileIcon {
                icon: '∞',
                color: "#654ca7",
            },
        );
        m.insert(
            "mobi",
            super::FileIcon {
                icon: '',
                color: "#755836",
            },
        );
        m.insert(
            "mojo",
            super::FileIcon {
                icon: '',
                color: "#bf3917",
            },
        );
        m.insert(
            "🔥",
            super::FileIcon {
                icon: '',
                color: "#bf3917",
            },
        );
        m.insert(
            "mov",
            super::FileIcon {
                icon: '',
                color: "#7e4c10",
            },
        );
        m.insert(
            "mp3",
            super::FileIcon {
                icon: '',
                color: "#0075aa",
            },
        );
        m.insert(
            "mp4",
            super::FileIcon {
                icon: '',
                color: "#7e4c10",
            },
        );
        m.insert(
            "mpp",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "msf",
            super::FileIcon {
                icon: '',
                color: "#0e5ca9",
            },
        );
        m.insert(
            "mts",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "mustache",
            super::FileIcon {
                icon: '',
                color: "#975122",
            },
        );
        m.insert(
            "nfo",
            super::FileIcon {
                icon: '',
                color: "#333329",
            },
        );
        m.insert(
            "nim",
            super::FileIcon {
                icon: '',
                color: "#514700",
            },
        );
        m.insert(
            "nix",
            super::FileIcon {
                icon: '',
                color: "#3f5d72",
            },
        );
        m.insert(
            "nswag",
            super::FileIcon {
                icon: '',
                color: "#427516",
            },
        );
        m.insert(
            "nu",
            super::FileIcon {
                icon: '>',
                color: "#276f4e",
            },
        );
        m.insert(
            "o",
            super::FileIcon {
                icon: '',
                color: "#9F0500",
            },
        );
        m.insert(
            "obj",
            super::FileIcon {
                icon: '󰆧',
                color: "#5b5b5b",
            },
        );
        m.insert(
            "ogg",
            super::FileIcon {
                icon: '',
                color: "#005880",
            },
        );
        m.insert(
            "opus",
            super::FileIcon {
                icon: '',
                color: "#005880",
            },
        );
        m.insert(
            "org",
            super::FileIcon {
                icon: '',
                color: "#4f7166",
            },
        );
        m.insert(
            "otf",
            super::FileIcon {
                icon: '',
                color: "#2f2f2f",
            },
        );
        m.insert(
            "out",
            super::FileIcon {
                icon: '',
                color: "#9F0500",
            },
        );
        m.insert(
            "part",
            super::FileIcon {
                icon: '',
                color: "#226654",
            },
        );
        m.insert(
            "patch",
            super::FileIcon {
                icon: '',
                color: "#41535b",
            },
        );
        m.insert(
            "pck",
            super::FileIcon {
                icon: '',
                color: "#526064",
            },
        );
        m.insert(
            "pcm",
            super::FileIcon {
                icon: '',
                color: "#005880",
            },
        );
        m.insert(
            "pdf",
            super::FileIcon {
                icon: '',
                color: "#b30b00",
            },
        );
        m.insert(
            "php",
            super::FileIcon {
                icon: '',
                color: "#6b4d83",
            },
        );
        m.insert(
            "pl",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "pls",
            super::FileIcon {
                icon: '󰲹',
                color: "#764a57",
            },
        );
        m.insert(
            "ply",
            super::FileIcon {
                icon: '󰆧',
                color: "#5b5b5b",
            },
        );
        m.insert(
            "pm",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "png",
            super::FileIcon {
                icon: '',
                color: "#6b4d83",
            },
        );
        m.insert(
            "po",
            super::FileIcon {
                icon: '',
                color: "#1c708e",
            },
        );
        m.insert(
            "pot",
            super::FileIcon {
                icon: '',
                color: "#1c708e",
            },
        );
        m.insert(
            "pp",
            super::FileIcon {
                icon: '',
                color: "#80530d",
            },
        );
        m.insert(
            "ppt",
            super::FileIcon {
                icon: '󰈧',
                color: "#983826",
            },
        );
        m.insert(
            "prisma",
            super::FileIcon {
                icon: '',
                color: "#444da2",
            },
        );
        m.insert(
            "pro",
            super::FileIcon {
                icon: '',
                color: "#725c2a",
            },
        );
        m.insert(
            "ps1",
            super::FileIcon {
                icon: '󰨊',
                color: "#325698",
            },
        );
        m.insert(
            "psd1",
            super::FileIcon {
                icon: '󰨊',
                color: "#4f5893",
            },
        );
        m.insert(
            "psm1",
            super::FileIcon {
                icon: '󰨊',
                color: "#4f5893",
            },
        );
        m.insert(
            "psb",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "psd",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "pub",
            super::FileIcon {
                icon: '󰷖',
                color: "#4c422f",
            },
        );
        m.insert(
            "pxd",
            super::FileIcon {
                icon: '',
                color: "#3c6f98",
            },
        );
        m.insert(
            "pxi",
            super::FileIcon {
                icon: '',
                color: "#3c6f98",
            },
        );
        m.insert(
            "py",
            super::FileIcon {
                icon: '',
                color: "#805e02",
            },
        );
        m.insert(
            "pyc",
            super::FileIcon {
                icon: '',
                color: "#332d1d",
            },
        );
        m.insert(
            "pyd",
            super::FileIcon {
                icon: '',
                color: "#332d1d",
            },
        );
        m.insert(
            "pyi",
            super::FileIcon {
                icon: '',
                color: "#805e02",
            },
        );
        m.insert(
            "pyo",
            super::FileIcon {
                icon: '',
                color: "#332d1d",
            },
        );
        m.insert(
            "pyw",
            super::FileIcon {
                icon: '',
                color: "#3c6f98",
            },
        );
        m.insert(
            "pyx",
            super::FileIcon {
                icon: '',
                color: "#3c6f98",
            },
        );
        m.insert(
            "qm",
            super::FileIcon {
                icon: '',
                color: "#1c708e",
            },
        );
        m.insert(
            "qml",
            super::FileIcon {
                icon: '',
                color: "#2b8937",
            },
        );
        m.insert(
            "qrc",
            super::FileIcon {
                icon: '',
                color: "#2b8937",
            },
        );
        m.insert(
            "qss",
            super::FileIcon {
                icon: '',
                color: "#2b8937",
            },
        );
        m.insert(
            "query",
            super::FileIcon {
                icon: '',
                color: "#607035",
            },
        );
        m.insert(
            "r",
            super::FileIcon {
                icon: '󰟔',
                color: "#1a4c8c",
            },
        );
        m.insert(
            "rake",
            super::FileIcon {
                icon: '',
                color: "#701516",
            },
        );
        m.insert(
            "rar",
            super::FileIcon {
                icon: '',
                color: "#76520c",
            },
        );
        m.insert(
            "razor",
            super::FileIcon {
                icon: '󱦘',
                color: "#512bd4",
            },
        );
        m.insert(
            "rb",
            super::FileIcon {
                icon: '',
                color: "#701516",
            },
        );
        m.insert(
            "res",
            super::FileIcon {
                icon: '',
                color: "#992e33",
            },
        );
        m.insert(
            "resi",
            super::FileIcon {
                icon: '',
                color: "#a33759",
            },
        );
        m.insert(
            "rlib",
            super::FileIcon {
                icon: '',
                color: "#6f5242",
            },
        );
        m.insert(
            "rmd",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "rproj",
            super::FileIcon {
                icon: '󰗆',
                color: "#286844",
            },
        );
        m.insert(
            "rs",
            super::FileIcon {
                icon: '',
                color: "#6f5242",
            },
        );
        m.insert(
            "rss",
            super::FileIcon {
                icon: '',
                color: "#7e4e1e",
            },
        );
        m.insert(
            "sass",
            super::FileIcon {
                icon: '',
                color: "#a33759",
            },
        );
        m.insert(
            "sbt",
            super::FileIcon {
                icon: '',
                color: "#992e33",
            },
        );
        m.insert(
            "scad",
            super::FileIcon {
                icon: '',
                color: "#53480f",
            },
        );
        m.insert(
            "scala",
            super::FileIcon {
                icon: '',
                color: "#992e33",
            },
        );
        m.insert(
            "sc",
            super::FileIcon {
                icon: '',
                color: "#992e33",
            },
        );
        m.insert(
            "scm",
            super::FileIcon {
                icon: '󰘧',
                color: "#303030",
            },
        );
        m.insert(
            "scss",
            super::FileIcon {
                icon: '',
                color: "#a33759",
            },
        );
        m.insert(
            "sh",
            super::FileIcon {
                icon: '',
                color: "#3a4446",
            },
        );
        m.insert(
            "sha1",
            super::FileIcon {
                icon: '󰕥',
                color: "#5d5975",
            },
        );
        m.insert(
            "sha224",
            super::FileIcon {
                icon: '󰕥',
                color: "#5d5975",
            },
        );
        m.insert(
            "sha256",
            super::FileIcon {
                icon: '󰕥',
                color: "#5d5975",
            },
        );
        m.insert(
            "sha384",
            super::FileIcon {
                icon: '󰕥',
                color: "#5d5975",
            },
        );
        m.insert(
            "sha512",
            super::FileIcon {
                icon: '󰕥',
                color: "#5d5975",
            },
        );
        m.insert(
            "sig",
            super::FileIcon {
                icon: 'λ',
                color: "#975122",
            },
        );
        m.insert(
            "signature",
            super::FileIcon {
                icon: 'λ',
                color: "#975122",
            },
        );
        m.insert(
            "skp",
            super::FileIcon {
                icon: '󰻫',
                color: "#576342",
            },
        );
        m.insert(
            "sldasm",
            super::FileIcon {
                icon: '󰻫',
                color: "#576342",
            },
        );
        m.insert(
            "sldprt",
            super::FileIcon {
                icon: '󰻫',
                color: "#576342",
            },
        );
        m.insert(
            "slim",
            super::FileIcon {
                icon: '',
                color: "#aa391c",
            },
        );
        m.insert(
            "sln",
            super::FileIcon {
                icon: '',
                color: "#643995",
            },
        );
        m.insert(
            "slvs",
            super::FileIcon {
                icon: '󰻫',
                color: "#576342",
            },
        );
        m.insert(
            "sml",
            super::FileIcon {
                icon: 'λ',
                color: "#975122",
            },
        );
        m.insert(
            "so",
            super::FileIcon {
                icon: '',
                color: "#494a47",
            },
        );
        m.insert(
            "sol",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "spec.js",
            super::FileIcon {
                icon: '',
                color: "#666620",
            },
        );
        m.insert(
            "spec.jsx",
            super::FileIcon {
                icon: '',
                color: "#158197",
            },
        );
        m.insert(
            "spec.ts",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "spec.tsx",
            super::FileIcon {
                icon: '',
                color: "#1354bf",
            },
        );
        m.insert(
            "sql",
            super::FileIcon {
                icon: '',
                color: "#494848",
            },
        );
        m.insert(
            "sqlite",
            super::FileIcon {
                icon: '',
                color: "#494848",
            },
        );
        m.insert(
            "sqlite3",
            super::FileIcon {
                icon: '',
                color: "#494848",
            },
        );
        m.insert(
            "srt",
            super::FileIcon {
                icon: '󰨖',
                color: "#805c0a",
            },
        );
        m.insert(
            "ssa",
            super::FileIcon {
                icon: '󰨖',
                color: "#805c0a",
            },
        );
        m.insert(
            "stl",
            super::FileIcon {
                icon: '󰆧',
                color: "#5b5b5b",
            },
        );
        m.insert(
            "strings",
            super::FileIcon {
                icon: '',
                color: "#1c708e",
            },
        );
        m.insert(
            "ste",
            super::FileIcon {
                icon: '󰻫',
                color: "#576342",
            },
        );
        m.insert(
            "step",
            super::FileIcon {
                icon: '󰻫',
                color: "#576342",
            },
        );
        m.insert(
            "stp",
            super::FileIcon {
                icon: '󰻫',
                color: "#576342",
            },
        );
        m.insert(
            "styl",
            super::FileIcon {
                icon: '',
                color: "#466024",
            },
        );
        m.insert(
            "sub",
            super::FileIcon {
                icon: '󰨖',
                color: "#805c0a",
            },
        );
        m.insert(
            "sublime",
            super::FileIcon {
                icon: '',
                color: "#975122",
            },
        );
        m.insert(
            "suo",
            super::FileIcon {
                icon: '',
                color: "#643995",
            },
        );
        m.insert(
            "sv",
            super::FileIcon {
                icon: '󰍛',
                color: "#017226",
            },
        );
        m.insert(
            "svelte",
            super::FileIcon {
                icon: '',
                color: "#bf2e00",
            },
        );
        m.insert(
            "svh",
            super::FileIcon {
                icon: '󰍛',
                color: "#017226",
            },
        );
        m.insert(
            "svg",
            super::FileIcon {
                icon: '󰜡',
                color: "#80581e",
            },
        );
        m.insert(
            "swift",
            super::FileIcon {
                icon: '',
                color: "#975122",
            },
        );
        m.insert(
            "t",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "tbc",
            super::FileIcon {
                icon: '󰛓',
                color: "#1e5cb3",
            },
        );
        m.insert(
            "tcl",
            super::FileIcon {
                icon: '󰛓',
                color: "#1e5cb3",
            },
        );
        m.insert(
            "templ",
            super::FileIcon {
                icon: '',
                color: "#6e5e18",
            },
        );
        m.insert(
            "terminal",
            super::FileIcon {
                icon: '',
                color: "#217929",
            },
        );
        m.insert(
            "test.js",
            super::FileIcon {
                icon: '',
                color: "#666620",
            },
        );
        m.insert(
            "test.jsx",
            super::FileIcon {
                icon: '',
                color: "#158197",
            },
        );
        m.insert(
            "test.ts",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "test.tsx",
            super::FileIcon {
                icon: '',
                color: "#1354bf",
            },
        );
        m.insert(
            "tex",
            super::FileIcon {
                icon: '',
                color: "#3D6117",
            },
        );
        m.insert(
            "tf",
            super::FileIcon {
                icon: '',
                color: "#4732af",
            },
        );
        m.insert(
            "tfvars",
            super::FileIcon {
                icon: '',
                color: "#4732af",
            },
        );
        m.insert(
            "tgz",
            super::FileIcon {
                icon: '',
                color: "#76520c",
            },
        );
        m.insert(
            "tmux",
            super::FileIcon {
                icon: '',
                color: "#0f8c13",
            },
        );
        m.insert(
            "toml",
            super::FileIcon {
                icon: '',
                color: "#753219",
            },
        );
        m.insert(
            "torrent",
            super::FileIcon {
                icon: '',
                color: "#226654",
            },
        );
        m.insert(
            "tres",
            super::FileIcon {
                icon: '',
                color: "#526064",
            },
        );
        m.insert(
            "ts",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "tscn",
            super::FileIcon {
                icon: '',
                color: "#526064",
            },
        );
        m.insert(
            "tsconfig",
            super::FileIcon {
                icon: '',
                color: "#aa5a00",
            },
        );
        m.insert(
            "tsx",
            super::FileIcon {
                icon: '',
                color: "#1354bf",
            },
        );
        m.insert(
            "ttf",
            super::FileIcon {
                icon: '',
                color: "#2f2f2f",
            },
        );
        m.insert(
            "twig",
            super::FileIcon {
                icon: '',
                color: "#466024",
            },
        );
        m.insert(
            "txz",
            super::FileIcon {
                icon: '',
                color: "#76520c",
            },
        );
        m.insert(
            "typoscript",
            super::FileIcon {
                icon: '',
                color: "#aa5a00",
            },
        );
        m.insert(
            "txt",
            super::FileIcon {
                icon: '󰈙',
                color: "#447028",
            },
        );
        m.insert(
            "ui",
            super::FileIcon {
                icon: '',
                color: "#015BF0",
            },
        );
        m.insert(
            "v",
            super::FileIcon {
                icon: '󰍛',
                color: "#017226",
            },
        );
        m.insert(
            "vala",
            super::FileIcon {
                icon: '',
                color: "#562b86",
            },
        );
        m.insert(
            "vh",
            super::FileIcon {
                icon: '󰍛',
                color: "#017226",
            },
        );
        m.insert(
            "vhd",
            super::FileIcon {
                icon: '󰍛',
                color: "#017226",
            },
        );
        m.insert(
            "vhdl",
            super::FileIcon {
                icon: '󰍛',
                color: "#017226",
            },
        );
        m.insert(
            "vim",
            super::FileIcon {
                icon: '',
                color: "#017226",
            },
        );
        m.insert(
            "vsh",
            super::FileIcon {
                icon: '',
                color: "#3e5a7f",
            },
        );
        m.insert(
            "vsix",
            super::FileIcon {
                icon: '',
                color: "#643995",
            },
        );
        m.insert(
            "vue",
            super::FileIcon {
                icon: '',
                color: "#466024",
            },
        );
        m.insert(
            "wasm",
            super::FileIcon {
                icon: '',
                color: "#4539a4",
            },
        );
        m.insert(
            "wav",
            super::FileIcon {
                icon: '',
                color: "#0075aa",
            },
        );
        m.insert(
            "webm",
            super::FileIcon {
                icon: '',
                color: "#7e4c10",
            },
        );
        m.insert(
            "webmanifest",
            super::FileIcon {
                icon: '',
                color: "#504b1e",
            },
        );
        m.insert(
            "webp",
            super::FileIcon {
                icon: '',
                color: "#6b4d83",
            },
        );
        m.insert(
            "webpack",
            super::FileIcon {
                icon: '󰜫',
                color: "#36677c",
            },
        );
        m.insert(
            "wma",
            super::FileIcon {
                icon: '',
                color: "#0075aa",
            },
        );
        m.insert(
            "woff",
            super::FileIcon {
                icon: '',
                color: "#2f2f2f",
            },
        );
        m.insert(
            "woff2",
            super::FileIcon {
                icon: '',
                color: "#2f2f2f",
            },
        );
        m.insert(
            "wrl",
            super::FileIcon {
                icon: '󰆧',
                color: "#5b5b5b",
            },
        );
        m.insert(
            "wrz",
            super::FileIcon {
                icon: '󰆧',
                color: "#5b5b5b",
            },
        );
        m.insert(
            "wv",
            super::FileIcon {
                icon: '',
                color: "#0075aa",
            },
        );
        m.insert(
            "wvc",
            super::FileIcon {
                icon: '',
                color: "#0075aa",
            },
        );
        m.insert(
            "x",
            super::FileIcon {
                icon: '',
                color: "#3b69aa",
            },
        );
        m.insert(
            "xm",
            super::FileIcon {
                icon: '',
                color: "#36677c",
            },
        );
        m.insert(
            "xaml",
            super::FileIcon {
                icon: '󰙳',
                color: "#512bd4",
            },
        );
        m.insert(
            "xcf",
            super::FileIcon {
                icon: '',
                color: "#4a4434",
            },
        );
        m.insert(
            "xcplayground",
            super::FileIcon {
                icon: '',
                color: "#975122",
            },
        );
        m.insert(
            "xcstrings",
            super::FileIcon {
                icon: '',
                color: "#1c708e",
            },
        );
        m.insert(
            "xls",
            super::FileIcon {
                icon: '󰈛',
                color: "#207245",
            },
        );
        m.insert(
            "xlsx",
            super::FileIcon {
                icon: '󰈛',
                color: "#207245",
            },
        );
        m.insert(
            "xml",
            super::FileIcon {
                icon: '󰗀',
                color: "#975122",
            },
        );
        m.insert(
            "xpi",
            super::FileIcon {
                icon: '',
                color: "#bf1401",
            },
        );
        m.insert(
            "xul",
            super::FileIcon {
                icon: '',
                color: "#975122",
            },
        );
        m.insert(
            "xz",
            super::FileIcon {
                icon: '',
                color: "#76520c",
            },
        );
        m.insert(
            "yaml",
            super::FileIcon {
                icon: '',
                color: "#526064",
            },
        );
        m.insert(
            "yml",
            super::FileIcon {
                icon: '',
                color: "#526064",
            },
        );
        m.insert(
            "zig",
            super::FileIcon {
                icon: '',
                color: "#7b4d0e",
            },
        );
        m.insert(
            "zip",
            super::FileIcon {
                icon: '',
                color: "#76520c",
            },
        );
        m.insert(
            "zsh",
            super::FileIcon {
                icon: '',
                color: "#447028",
            },
        );
        m.insert(
            "zst",
            super::FileIcon {
                icon: '',
                color: "#76520c",
            },
        );
        m
    };
}
