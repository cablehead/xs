use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    pub static ref ICONS_MAP: HashMap<&'static str, super::FileIcon> = {
        let mut m = HashMap::new();
        m.insert(
            ".babelrc",
            super::FileIcon {
                icon: '',
                color: "#cbcb41",
            },
        );
        m.insert(
            ".bash_profile",
            super::FileIcon {
                icon: '',
                color: "#89e051",
            },
        );
        m.insert(
            ".bashrc",
            super::FileIcon {
                icon: '',
                color: "#89e051",
            },
        );
        m.insert(
            ".dockerignore",
            super::FileIcon {
                icon: '󰡨',
                color: "#458ee6",
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
                color: "#fff2f2",
            },
        );
        m.insert(
            ".env",
            super::FileIcon {
                icon: '',
                color: "#faf743",
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
                color: "#f54d27",
            },
        );
        m.insert(
            ".gitattributes",
            super::FileIcon {
                icon: '',
                color: "#f54d27",
            },
        );
        m.insert(
            ".gitconfig",
            super::FileIcon {
                icon: '',
                color: "#f54d27",
            },
        );
        m.insert(
            ".gitignore",
            super::FileIcon {
                icon: '',
                color: "#f54d27",
            },
        );
        m.insert(
            ".gitlab-ci.yml",
            super::FileIcon {
                icon: '',
                color: "#e24329",
            },
        );
        m.insert(
            ".gitmodules",
            super::FileIcon {
                icon: '',
                color: "#f54d27",
            },
        );
        m.insert(
            ".gtkrc-2.0",
            super::FileIcon {
                icon: '',
                color: "#ffffff",
            },
        );
        m.insert(
            ".gvimrc",
            super::FileIcon {
                icon: '',
                color: "#019833",
            },
        );
        m.insert(
            ".justfile",
            super::FileIcon {
                icon: '',
                color: "#6d8086",
            },
        );
        m.insert(
            ".luaurc",
            super::FileIcon {
                icon: '',
                color: "#00a2ff",
            },
        );
        m.insert(
            ".mailmap",
            super::FileIcon {
                icon: '󰊢',
                color: "#f54d27",
            },
        );
        m.insert(
            ".npmignore",
            super::FileIcon {
                icon: '',
                color: "#E8274B",
            },
        );
        m.insert(
            ".npmrc",
            super::FileIcon {
                icon: '',
                color: "#E8274B",
            },
        );
        m.insert(
            ".nuxtrc",
            super::FileIcon {
                icon: '󱄆',
                color: "#00c58e",
            },
        );
        m.insert(
            ".nvmrc",
            super::FileIcon {
                icon: '',
                color: "#5FA04E",
            },
        );
        m.insert(
            ".prettierrc",
            super::FileIcon {
                icon: '',
                color: "#4285F4",
            },
        );
        m.insert(
            ".prettierrc.json",
            super::FileIcon {
                icon: '',
                color: "#4285F4",
            },
        );
        m.insert(
            ".prettierrc.json5",
            super::FileIcon {
                icon: '',
                color: "#4285F4",
            },
        );
        m.insert(
            ".prettierrc.toml",
            super::FileIcon {
                icon: '',
                color: "#4285F4",
            },
        );
        m.insert(
            ".prettierrc.yaml",
            super::FileIcon {
                icon: '',
                color: "#4285F4",
            },
        );
        m.insert(
            ".prettierrc.yml",
            super::FileIcon {
                icon: '',
                color: "#4285F4",
            },
        );
        m.insert(
            ".prettierignore",
            super::FileIcon {
                icon: '',
                color: "#4285F4",
            },
        );
        m.insert(
            ".settings.json",
            super::FileIcon {
                icon: '',
                color: "#854CC7",
            },
        );
        m.insert(
            ".SRCINFO",
            super::FileIcon {
                icon: '󰣇',
                color: "#0f94d2",
            },
        );
        m.insert(
            ".vimrc",
            super::FileIcon {
                icon: '',
                color: "#019833",
            },
        );
        m.insert(
            ".Xauthority",
            super::FileIcon {
                icon: '',
                color: "#e54d18",
            },
        );
        m.insert(
            ".xinitrc",
            super::FileIcon {
                icon: '',
                color: "#e54d18",
            },
        );
        m.insert(
            ".Xresources",
            super::FileIcon {
                icon: '',
                color: "#e54d18",
            },
        );
        m.insert(
            ".xsession",
            super::FileIcon {
                icon: '',
                color: "#e54d18",
            },
        );
        m.insert(
            ".zprofile",
            super::FileIcon {
                icon: '',
                color: "#89e051",
            },
        );
        m.insert(
            ".zshenv",
            super::FileIcon {
                icon: '',
                color: "#89e051",
            },
        );
        m.insert(
            ".zshrc",
            super::FileIcon {
                icon: '',
                color: "#89e051",
            },
        );
        m.insert(
            "_gvimrc",
            super::FileIcon {
                icon: '',
                color: "#019833",
            },
        );
        m.insert(
            "_vimrc",
            super::FileIcon {
                icon: '',
                color: "#019833",
            },
        );
        m.insert(
            "R",
            super::FileIcon {
                icon: '󰟔',
                color: "#2266ba",
            },
        );
        m.insert(
            "avif",
            super::FileIcon {
                icon: '',
                color: "#a074c4",
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
                color: "#89e051",
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
                color: "#f69a1b",
            },
        );
        m.insert(
            "checkhealth",
            super::FileIcon {
                icon: '󰓙',
                color: "#75B4FB",
            },
        );
        m.insert(
            "cmakelists.txt",
            super::FileIcon {
                icon: '',
                color: "#6d8086",
            },
        );
        m.insert(
            "code_of_conduct",
            super::FileIcon {
                icon: '',
                color: "#E41662",
            },
        );
        m.insert(
            "code_of_conduct.md",
            super::FileIcon {
                icon: '',
                color: "#E41662",
            },
        );
        m.insert(
            "commit_editmsg",
            super::FileIcon {
                icon: '',
                color: "#f54d27",
            },
        );
        m.insert(
            "commitlint.config.js",
            super::FileIcon {
                icon: '󰜘',
                color: "#2b9689",
            },
        );
        m.insert(
            "commitlint.config.ts",
            super::FileIcon {
                icon: '󰜘',
                color: "#2b9689",
            },
        );
        m.insert(
            "compose.yaml",
            super::FileIcon {
                icon: '󰡨',
                color: "#458ee6",
            },
        );
        m.insert(
            "compose.yml",
            super::FileIcon {
                icon: '󰡨',
                color: "#458ee6",
            },
        );
        m.insert(
            "config",
            super::FileIcon {
                icon: '',
                color: "#6d8086",
            },
        );
        m.insert(
            "containerfile",
            super::FileIcon {
                icon: '󰡨',
                color: "#458ee6",
            },
        );
        m.insert(
            "copying",
            super::FileIcon {
                icon: '',
                color: "#cbcb41",
            },
        );
        m.insert(
            "copying.lesser",
            super::FileIcon {
                icon: '',
                color: "#cbcb41",
            },
        );
        m.insert(
            "docker-compose.yaml",
            super::FileIcon {
                icon: '󰡨',
                color: "#458ee6",
            },
        );
        m.insert(
            "docker-compose.yml",
            super::FileIcon {
                icon: '󰡨',
                color: "#458ee6",
            },
        );
        m.insert(
            "dockerfile",
            super::FileIcon {
                icon: '󰡨',
                color: "#458ee6",
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
                color: "#FF8700",
            },
        );
        m.insert(
            "favicon.ico",
            super::FileIcon {
                icon: '',
                color: "#cbcb41",
            },
        );
        m.insert(
            "fp-info-cache",
            super::FileIcon {
                icon: '',
                color: "#ffffff",
            },
        );
        m.insert(
            "fp-lib-table",
            super::FileIcon {
                icon: '',
                color: "#ffffff",
            },
        );
        m.insert(
            "FreeCAD.conf",
            super::FileIcon {
                icon: '',
                color: "#CB333B",
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
                color: "#6d8086",
            },
        );
        m.insert(
            "go.mod",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "go.sum",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "go.work",
            super::FileIcon {
                icon: '',
                color: "#519aba",
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
                color: "#4a687c",
            },
        );
        m.insert(
            "gruntfile.babel.js",
            super::FileIcon {
                icon: '',
                color: "#e37933",
            },
        );
        m.insert(
            "gruntfile.coffee",
            super::FileIcon {
                icon: '',
                color: "#e37933",
            },
        );
        m.insert(
            "gruntfile.js",
            super::FileIcon {
                icon: '',
                color: "#e37933",
            },
        );
        m.insert(
            "gruntfile.ts",
            super::FileIcon {
                icon: '',
                color: "#e37933",
            },
        );
        m.insert(
            "gtkrc",
            super::FileIcon {
                icon: '',
                color: "#ffffff",
            },
        );
        m.insert(
            "gulpfile.babel.js",
            super::FileIcon {
                icon: '',
                color: "#cc3e44",
            },
        );
        m.insert(
            "gulpfile.coffee",
            super::FileIcon {
                icon: '',
                color: "#cc3e44",
            },
        );
        m.insert(
            "gulpfile.js",
            super::FileIcon {
                icon: '',
                color: "#cc3e44",
            },
        );
        m.insert(
            "gulpfile.ts",
            super::FileIcon {
                icon: '',
                color: "#cc3e44",
            },
        );
        m.insert(
            "hypridle.conf",
            super::FileIcon {
                icon: '',
                color: "#00aaae",
            },
        );
        m.insert(
            "hyprland.conf",
            super::FileIcon {
                icon: '',
                color: "#00aaae",
            },
        );
        m.insert(
            "hyprlock.conf",
            super::FileIcon {
                icon: '',
                color: "#00aaae",
            },
        );
        m.insert(
            "i18n.config.js",
            super::FileIcon {
                icon: '󰗊',
                color: "#7986cb",
            },
        );
        m.insert(
            "i18n.config.ts",
            super::FileIcon {
                icon: '󰗊',
                color: "#7986cb",
            },
        );
        m.insert(
            "i3blocks.conf",
            super::FileIcon {
                icon: '',
                color: "#e8ebee",
            },
        );
        m.insert(
            "i3status.conf",
            super::FileIcon {
                icon: '',
                color: "#e8ebee",
            },
        );
        m.insert(
            "ionic.config.json",
            super::FileIcon {
                icon: '',
                color: "#4f8ff7",
            },
        );
        m.insert(
            "cantorrc",
            super::FileIcon {
                icon: '',
                color: "#1c99f3",
            },
        );
        m.insert(
            "justfile",
            super::FileIcon {
                icon: '',
                color: "#6d8086",
            },
        );
        m.insert(
            "kalgebrarc",
            super::FileIcon {
                icon: '',
                color: "#1c99f3",
            },
        );
        m.insert(
            "kdeglobals",
            super::FileIcon {
                icon: '',
                color: "#1c99f3",
            },
        );
        m.insert(
            "kdenlive-layoutsrc",
            super::FileIcon {
                icon: '',
                color: "#83b8f2",
            },
        );
        m.insert(
            "kdenliverc",
            super::FileIcon {
                icon: '',
                color: "#83b8f2",
            },
        );
        m.insert(
            "kritadisplayrc",
            super::FileIcon {
                icon: '',
                color: "#f245fb",
            },
        );
        m.insert(
            "kritarc",
            super::FileIcon {
                icon: '',
                color: "#f245fb",
            },
        );
        m.insert(
            "license",
            super::FileIcon {
                icon: '',
                color: "#d0bf41",
            },
        );
        m.insert(
            "license.md",
            super::FileIcon {
                icon: '',
                color: "#d0bf41",
            },
        );
        m.insert(
            "lxde-rc.xml",
            super::FileIcon {
                icon: '',
                color: "#909090",
            },
        );
        m.insert(
            "lxqt.conf",
            super::FileIcon {
                icon: '',
                color: "#0192d3",
            },
        );
        m.insert(
            "makefile",
            super::FileIcon {
                icon: '',
                color: "#6d8086",
            },
        );
        m.insert(
            "mix.lock",
            super::FileIcon {
                icon: '',
                color: "#a074c4",
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
                color: "#E8274B",
            },
        );
        m.insert(
            "nuxt.config.cjs",
            super::FileIcon {
                icon: '󱄆',
                color: "#00c58e",
            },
        );
        m.insert(
            "nuxt.config.js",
            super::FileIcon {
                icon: '󱄆',
                color: "#00c58e",
            },
        );
        m.insert(
            "nuxt.config.mjs",
            super::FileIcon {
                icon: '󱄆',
                color: "#00c58e",
            },
        );
        m.insert(
            "nuxt.config.ts",
            super::FileIcon {
                icon: '󱄆',
                color: "#00c58e",
            },
        );
        m.insert(
            "package.json",
            super::FileIcon {
                icon: '',
                color: "#e8274b",
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
                color: "#0f94d2",
            },
        );
        m.insert(
            "platformio.ini",
            super::FileIcon {
                icon: '',
                color: "#f6822b",
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
                color: "#4285F4",
            },
        );
        m.insert(
            "prettier.config.cjs",
            super::FileIcon {
                icon: '',
                color: "#4285F4",
            },
        );
        m.insert(
            "prettier.config.mjs",
            super::FileIcon {
                icon: '',
                color: "#4285F4",
            },
        );
        m.insert(
            "prettier.config.ts",
            super::FileIcon {
                icon: '',
                color: "#4285F4",
            },
        );
        m.insert(
            "procfile",
            super::FileIcon {
                icon: '',
                color: "#a074c4",
            },
        );
        m.insert(
            "PrusaSlicer.ini",
            super::FileIcon {
                icon: '',
                color: "#ec6b23",
            },
        );
        m.insert(
            "PrusaSlicerGcodeViewer.ini",
            super::FileIcon {
                icon: '',
                color: "#ec6b23",
            },
        );
        m.insert(
            "py.typed",
            super::FileIcon {
                icon: '',
                color: "#ffbc03",
            },
        );
        m.insert(
            "QtProject.conf",
            super::FileIcon {
                icon: '',
                color: "#40cd52",
            },
        );
        m.insert(
            "r",
            super::FileIcon {
                icon: '󰟔',
                color: "#2266ba",
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
                color: "#519aba",
            },
        );
        m.insert(
            "robots.txt",
            super::FileIcon {
                icon: '󰚩',
                color: "#5d7096",
            },
        );
        m.insert(
            "security",
            super::FileIcon {
                icon: '󰒃',
                color: "#BEC4C9",
            },
        );
        m.insert(
            "security.md",
            super::FileIcon {
                icon: '󰒃',
                color: "#BEC4C9",
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
                color: "#ff3e00",
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
                color: "#ffffff",
            },
        );
        m.insert(
            "tailwind.config.js",
            super::FileIcon {
                icon: '󱏿',
                color: "#20c2e3",
            },
        );
        m.insert(
            "tailwind.config.mjs",
            super::FileIcon {
                icon: '󱏿',
                color: "#20c2e3",
            },
        );
        m.insert(
            "tailwind.config.ts",
            super::FileIcon {
                icon: '󱏿',
                color: "#20c2e3",
            },
        );
        m.insert(
            "tmux.conf",
            super::FileIcon {
                icon: '',
                color: "#14ba19",
            },
        );
        m.insert(
            "tmux.conf.local",
            super::FileIcon {
                icon: '',
                color: "#14ba19",
            },
        );
        m.insert(
            "tsconfig.json",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "unlicense",
            super::FileIcon {
                icon: '',
                color: "#d0bf41",
            },
        );
        m.insert(
            "vagrantfile$",
            super::FileIcon {
                icon: '',
                color: "#1563FF",
            },
        );
        m.insert(
            "vlcrc",
            super::FileIcon {
                icon: '󰕼',
                color: "#ee7a00",
            },
        );
        m.insert(
            "vercel.json",
            super::FileIcon {
                icon: '▲',
                color: "#ffffff",
            },
        );
        m.insert(
            "webpack",
            super::FileIcon {
                icon: '󰜫',
                color: "#519aba",
            },
        );
        m.insert(
            "weston.ini",
            super::FileIcon {
                icon: '',
                color: "#ffbb01",
            },
        );
        m.insert(
            "workspace",
            super::FileIcon {
                icon: '',
                color: "#89e051",
            },
        );
        m.insert(
            "xmobarrc",
            super::FileIcon {
                icon: '',
                color: "#fd4d5d",
            },
        );
        m.insert(
            "xmobarrc.hs",
            super::FileIcon {
                icon: '',
                color: "#fd4d5d",
            },
        );
        m.insert(
            "xmonad.hs",
            super::FileIcon {
                icon: '',
                color: "#fd4d5d",
            },
        );
        m.insert(
            "xorg.conf",
            super::FileIcon {
                icon: '',
                color: "#e54d18",
            },
        );
        m.insert(
            "xsettingsd.conf",
            super::FileIcon {
                icon: '',
                color: "#e54d18",
            },
        );
        m.insert(
            "3gp",
            super::FileIcon {
                icon: '',
                color: "#FD971F",
            },
        );
        m.insert(
            "3mf",
            super::FileIcon {
                icon: '󰆧',
                color: "#888888",
            },
        );
        m.insert(
            "7z",
            super::FileIcon {
                icon: '',
                color: "#eca517",
            },
        );
        m.insert(
            "a",
            super::FileIcon {
                icon: '',
                color: "#dcddd6",
            },
        );
        m.insert(
            "aac",
            super::FileIcon {
                icon: '',
                color: "#00afff",
            },
        );
        m.insert(
            "aif",
            super::FileIcon {
                icon: '',
                color: "#00afff",
            },
        );
        m.insert(
            "aiff",
            super::FileIcon {
                icon: '',
                color: "#00afff",
            },
        );
        m.insert(
            "ape",
            super::FileIcon {
                icon: '',
                color: "#00afff",
            },
        );
        m.insert(
            "apl",
            super::FileIcon {
                icon: '⍝',
                color: "#ffa500",
            },
        );
        m.insert(
            "ai",
            super::FileIcon {
                icon: '',
                color: "#cbcb41",
            },
        );
        m.insert(
            "android",
            super::FileIcon {
                icon: '',
                color: "#34a853",
            },
        );
        m.insert(
            "apk",
            super::FileIcon {
                icon: '',
                color: "#34a853",
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
                color: "#6d8085",
            },
        );
        m.insert(
            "asc",
            super::FileIcon {
                icon: '󰦝',
                color: "#576d7f",
            },
        );
        m.insert(
            "ass",
            super::FileIcon {
                icon: '󰨖',
                color: "#ffb713",
            },
        );
        m.insert(
            "astro",
            super::FileIcon {
                icon: '',
                color: "#e23f67",
            },
        );
        m.insert(
            "awk",
            super::FileIcon {
                icon: '',
                color: "#4d5a5e",
            },
        );
        m.insert(
            "azcli",
            super::FileIcon {
                icon: '',
                color: "#0078d4",
            },
        );
        m.insert(
            "bak",
            super::FileIcon {
                icon: '󰁯',
                color: "#6d8086",
            },
        );
        m.insert(
            "bash",
            super::FileIcon {
                icon: '',
                color: "#89e051",
            },
        );
        m.insert(
            "bat",
            super::FileIcon {
                icon: '',
                color: "#C1F12E",
            },
        );
        m.insert(
            "bazel",
            super::FileIcon {
                icon: '',
                color: "#89e051",
            },
        );
        m.insert(
            "bib",
            super::FileIcon {
                icon: '󱉟',
                color: "#cbcb41",
            },
        );
        m.insert(
            "bicep",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "bicepparam",
            super::FileIcon {
                icon: '',
                color: "#9f74b3",
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
                color: "#f05340",
            },
        );
        m.insert(
            "blend",
            super::FileIcon {
                icon: '󰂫',
                color: "#ea7600",
            },
        );
        m.insert(
            "bmp",
            super::FileIcon {
                icon: '',
                color: "#a074c4",
            },
        );
        m.insert(
            "bqn",
            super::FileIcon {
                icon: '⎉',
                color: "#2b7067",
            },
        );
        m.insert(
            "blp",
            super::FileIcon {
                icon: '󰺾',
                color: "#5796E2",
            },
        );
        m.insert(
            "brep",
            super::FileIcon {
                icon: '󰻫',
                color: "#839463",
            },
        );
        m.insert(
            "bz",
            super::FileIcon {
                icon: '',
                color: "#eca517",
            },
        );
        m.insert(
            "bz2",
            super::FileIcon {
                icon: '',
                color: "#eca517",
            },
        );
        m.insert(
            "bz3",
            super::FileIcon {
                icon: '',
                color: "#eca517",
            },
        );
        m.insert(
            "bzl",
            super::FileIcon {
                icon: '',
                color: "#89e051",
            },
        );
        m.insert(
            "c",
            super::FileIcon {
                icon: '',
                color: "#599eff",
            },
        );
        m.insert(
            "c++",
            super::FileIcon {
                icon: '',
                color: "#f34b7d",
            },
        );
        m.insert(
            "cache",
            super::FileIcon {
                icon: '',
                color: "#ffffff",
            },
        );
        m.insert(
            "cast",
            super::FileIcon {
                icon: '',
                color: "#FD971F",
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
                color: "#f34b7d",
            },
        );
        m.insert(
            "ccm",
            super::FileIcon {
                icon: '',
                color: "#f34b7d",
            },
        );
        m.insert(
            "cfg",
            super::FileIcon {
                icon: '',
                color: "#6d8086",
            },
        );
        m.insert(
            "cjs",
            super::FileIcon {
                icon: '',
                color: "#cbcb41",
            },
        );
        m.insert(
            "clj",
            super::FileIcon {
                icon: '',
                color: "#8dc149",
            },
        );
        m.insert(
            "cljc",
            super::FileIcon {
                icon: '',
                color: "#8dc149",
            },
        );
        m.insert(
            "cljs",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "cljd",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "cmake",
            super::FileIcon {
                icon: '',
                color: "#6d8086",
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
                color: "#cbcb41",
            },
        );
        m.insert(
            "conf",
            super::FileIcon {
                icon: '',
                color: "#6d8086",
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
                color: "#965824",
            },
        );
        m.insert(
            "cp",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "cpp",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "cppm",
            super::FileIcon {
                icon: '',
                color: "#519aba",
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
                color: "#c8c8c8",
            },
        );
        m.insert(
            "crdownload",
            super::FileIcon {
                icon: '',
                color: "#44cda8",
            },
        );
        m.insert(
            "cs",
            super::FileIcon {
                icon: '󰌛',
                color: "#596706",
            },
        );
        m.insert(
            "csh",
            super::FileIcon {
                icon: '',
                color: "#4d5a5e",
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
                color: "#cbcb41",
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
                color: "#42a5f5",
            },
        );
        m.insert(
            "csv",
            super::FileIcon {
                icon: '',
                color: "#89e051",
            },
        );
        m.insert(
            "cts",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "cu",
            super::FileIcon {
                icon: '',
                color: "#89e051",
            },
        );
        m.insert(
            "cue",
            super::FileIcon {
                icon: '󰲹',
                color: "#ed95ae",
            },
        );
        m.insert(
            "cuh",
            super::FileIcon {
                icon: '',
                color: "#a074c4",
            },
        );
        m.insert(
            "cxx",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "cxxm",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "d",
            super::FileIcon {
                icon: '',
                color: "#427819",
            },
        );
        m.insert(
            "d.ts",
            super::FileIcon {
                icon: '',
                color: "#d59855",
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
                color: "#dad8d8",
            },
        );
        m.insert(
            "dconf",
            super::FileIcon {
                icon: '',
                color: "#ffffff",
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
                color: "#458ee6",
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
                color: "#30638e",
            },
        );
        m.insert(
            "download",
            super::FileIcon {
                icon: '',
                color: "#44cda8",
            },
        );
        m.insert(
            "drl",
            super::FileIcon {
                icon: '',
                color: "#ffafaf",
            },
        );
        m.insert(
            "dropbox",
            super::FileIcon {
                icon: '',
                color: "#0061FE",
            },
        );
        m.insert(
            "dump",
            super::FileIcon {
                icon: '',
                color: "#dad8d8",
            },
        );
        m.insert(
            "dwg",
            super::FileIcon {
                icon: '󰻫',
                color: "#839463",
            },
        );
        m.insert(
            "dxf",
            super::FileIcon {
                icon: '󰻫',
                color: "#839463",
            },
        );
        m.insert(
            "ebook",
            super::FileIcon {
                icon: '',
                color: "#eab16d",
            },
        );
        m.insert(
            "edn",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "eex",
            super::FileIcon {
                icon: '',
                color: "#a074c4",
            },
        );
        m.insert(
            "ejs",
            super::FileIcon {
                icon: '',
                color: "#cbcb41",
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
                color: "#8172be",
            },
        );
        m.insert(
            "elc",
            super::FileIcon {
                icon: '',
                color: "#8172be",
            },
        );
        m.insert(
            "elm",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "eln",
            super::FileIcon {
                icon: '',
                color: "#8172be",
            },
        );
        m.insert(
            "env",
            super::FileIcon {
                icon: '',
                color: "#faf743",
            },
        );
        m.insert(
            "eot",
            super::FileIcon {
                icon: '',
                color: "#ECECEC",
            },
        );
        m.insert(
            "epp",
            super::FileIcon {
                icon: '',
                color: "#FFA61A",
            },
        );
        m.insert(
            "epub",
            super::FileIcon {
                icon: '',
                color: "#eab16d",
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
                color: "#B83998",
            },
        );
        m.insert(
            "ex",
            super::FileIcon {
                icon: '',
                color: "#a074c4",
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
                color: "#a074c4",
            },
        );
        m.insert(
            "f#",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "f3d",
            super::FileIcon {
                icon: '󰻫',
                color: "#839463",
            },
        );
        m.insert(
            "f90",
            super::FileIcon {
                icon: '󱈚',
                color: "#734f96",
            },
        );
        m.insert(
            "fbx",
            super::FileIcon {
                icon: '󰆧',
                color: "#888888",
            },
        );
        m.insert(
            "fcbak",
            super::FileIcon {
                icon: '',
                color: "#cb333b",
            },
        );
        m.insert(
            "fcmacro",
            super::FileIcon {
                icon: '',
                color: "#cb333b",
            },
        );
        m.insert(
            "fcmat",
            super::FileIcon {
                icon: '',
                color: "#cb333b",
            },
        );
        m.insert(
            "fcparam",
            super::FileIcon {
                icon: '',
                color: "#cb333b",
            },
        );
        m.insert(
            "fcscript",
            super::FileIcon {
                icon: '',
                color: "#cb333b",
            },
        );
        m.insert(
            "fcstd",
            super::FileIcon {
                icon: '',
                color: "#cb333b",
            },
        );
        m.insert(
            "fcstd1",
            super::FileIcon {
                icon: '',
                color: "#cb333b",
            },
        );
        m.insert(
            "fctb",
            super::FileIcon {
                icon: '',
                color: "#cb333b",
            },
        );
        m.insert(
            "fctl",
            super::FileIcon {
                icon: '',
                color: "#cb333b",
            },
        );
        m.insert(
            "fdmdownload",
            super::FileIcon {
                icon: '',
                color: "#44cda8",
            },
        );
        m.insert(
            "flac",
            super::FileIcon {
                icon: '',
                color: "#0075aa",
            },
        );
        m.insert(
            "flc",
            super::FileIcon {
                icon: '',
                color: "#ECECEC",
            },
        );
        m.insert(
            "flf",
            super::FileIcon {
                icon: '',
                color: "#ECECEC",
            },
        );
        m.insert(
            "fnl",
            super::FileIcon {
                icon: '',
                color: "#fff3d7",
            },
        );
        m.insert(
            "fish",
            super::FileIcon {
                icon: '',
                color: "#4d5a5e",
            },
        );
        m.insert(
            "fs",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "fsi",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "fsscript",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "fsx",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "gcode",
            super::FileIcon {
                icon: '󰐫',
                color: "#1471ad",
            },
        );
        m.insert(
            "gd",
            super::FileIcon {
                icon: '',
                color: "#6d8086",
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
                color: "#a074c4",
            },
        );
        m.insert(
            "git",
            super::FileIcon {
                icon: '',
                color: "#F14C28",
            },
        );
        m.insert(
            "glb",
            super::FileIcon {
                icon: '',
                color: "#FFB13B",
            },
        );
        m.insert(
            "gleam",
            super::FileIcon {
                icon: '',
                color: "#ffaff3",
            },
        );
        m.insert(
            "gnumakefile",
            super::FileIcon {
                icon: '',
                color: "#6d8086",
            },
        );
        m.insert(
            "go",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "godot",
            super::FileIcon {
                icon: '',
                color: "#6d8086",
            },
        );
        m.insert(
            "gql",
            super::FileIcon {
                icon: '',
                color: "#e535ab",
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
                color: "#e535ab",
            },
        );
        m.insert(
            "gresource",
            super::FileIcon {
                icon: '',
                color: "#ffffff",
            },
        );
        m.insert(
            "gv",
            super::FileIcon {
                icon: '󱁉',
                color: "#30638e",
            },
        );
        m.insert(
            "gz",
            super::FileIcon {
                icon: '',
                color: "#eca517",
            },
        );
        m.insert(
            "h",
            super::FileIcon {
                icon: '',
                color: "#a074c4",
            },
        );
        m.insert(
            "haml",
            super::FileIcon {
                icon: '',
                color: "#eaeae1",
            },
        );
        m.insert(
            "hx",
            super::FileIcon {
                icon: '',
                color: "#ea8220",
            },
        );
        m.insert(
            "hbs",
            super::FileIcon {
                icon: '',
                color: "#f0772b",
            },
        );
        m.insert(
            "hex",
            super::FileIcon {
                icon: '',
                color: "#2e63ff",
            },
        );
        m.insert(
            "heex",
            super::FileIcon {
                icon: '',
                color: "#a074c4",
            },
        );
        m.insert(
            "hh",
            super::FileIcon {
                icon: '',
                color: "#a074c4",
            },
        );
        m.insert(
            "hpp",
            super::FileIcon {
                icon: '',
                color: "#a074c4",
            },
        );
        m.insert(
            "hrl",
            super::FileIcon {
                icon: '',
                color: "#B83998",
            },
        );
        m.insert(
            "hs",
            super::FileIcon {
                icon: '',
                color: "#a074c4",
            },
        );
        m.insert(
            "htm",
            super::FileIcon {
                icon: '',
                color: "#e34c26",
            },
        );
        m.insert(
            "html",
            super::FileIcon {
                icon: '',
                color: "#e44d26",
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
                color: "#ff0288",
            },
        );
        m.insert(
            "hxx",
            super::FileIcon {
                icon: '',
                color: "#a074c4",
            },
        );
        m.insert(
            "ixx",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "ico",
            super::FileIcon {
                icon: '',
                color: "#cbcb41",
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
                color: "#839463",
            },
        );
        m.insert(
            "ige",
            super::FileIcon {
                icon: '󰻫',
                color: "#839463",
            },
        );
        m.insert(
            "iges",
            super::FileIcon {
                icon: '󰻫',
                color: "#839463",
            },
        );
        m.insert(
            "igs",
            super::FileIcon {
                icon: '󰻫',
                color: "#839463",
            },
        );
        m.insert(
            "image",
            super::FileIcon {
                icon: '',
                color: "#d0bec8",
            },
        );
        m.insert(
            "img",
            super::FileIcon {
                icon: '',
                color: "#d0bec8",
            },
        );
        m.insert(
            "import",
            super::FileIcon {
                icon: '',
                color: "#ECECEC",
            },
        );
        m.insert(
            "info",
            super::FileIcon {
                icon: '',
                color: "#ffffcd",
            },
        );
        m.insert(
            "ini",
            super::FileIcon {
                icon: '',
                color: "#6d8086",
            },
        );
        m.insert(
            "ino",
            super::FileIcon {
                icon: '',
                color: "#56b6c2",
            },
        );
        m.insert(
            "iso",
            super::FileIcon {
                icon: '',
                color: "#d0bec8",
            },
        );
        m.insert(
            "ipynb",
            super::FileIcon {
                icon: '',
                color: "#51a0cf",
            },
        );
        m.insert(
            "java",
            super::FileIcon {
                icon: '',
                color: "#cc3e44",
            },
        );
        m.insert(
            "jl",
            super::FileIcon {
                icon: '',
                color: "#a270ba",
            },
        );
        m.insert(
            "jwmrc",
            super::FileIcon {
                icon: '',
                color: "#0078cd",
            },
        );
        m.insert(
            "jpeg",
            super::FileIcon {
                icon: '',
                color: "#a074c4",
            },
        );
        m.insert(
            "jpg",
            super::FileIcon {
                icon: '',
                color: "#a074c4",
            },
        );
        m.insert(
            "js",
            super::FileIcon {
                icon: '',
                color: "#cbcb41",
            },
        );
        m.insert(
            "json",
            super::FileIcon {
                icon: '',
                color: "#cbcb41",
            },
        );
        m.insert(
            "json5",
            super::FileIcon {
                icon: '',
                color: "#cbcb41",
            },
        );
        m.insert(
            "jsonc",
            super::FileIcon {
                icon: '',
                color: "#cbcb41",
            },
        );
        m.insert(
            "jsx",
            super::FileIcon {
                icon: '',
                color: "#20c2e3",
            },
        );
        m.insert(
            "jxl",
            super::FileIcon {
                icon: '',
                color: "#a074c4",
            },
        );
        m.insert(
            "kbx",
            super::FileIcon {
                icon: '󰯄',
                color: "#737672",
            },
        );
        m.insert(
            "kdb",
            super::FileIcon {
                icon: '',
                color: "#529b34",
            },
        );
        m.insert(
            "kdbx",
            super::FileIcon {
                icon: '',
                color: "#529b34",
            },
        );
        m.insert(
            "kdenlive",
            super::FileIcon {
                icon: '',
                color: "#83b8f2",
            },
        );
        m.insert(
            "kdenlivetitle",
            super::FileIcon {
                icon: '',
                color: "#83b8f2",
            },
        );
        m.insert(
            "kicad_dru",
            super::FileIcon {
                icon: '',
                color: "#ffffff",
            },
        );
        m.insert(
            "kicad_mod",
            super::FileIcon {
                icon: '',
                color: "#ffffff",
            },
        );
        m.insert(
            "kicad_pcb",
            super::FileIcon {
                icon: '',
                color: "#ffffff",
            },
        );
        m.insert(
            "kicad_prl",
            super::FileIcon {
                icon: '',
                color: "#ffffff",
            },
        );
        m.insert(
            "kicad_pro",
            super::FileIcon {
                icon: '',
                color: "#ffffff",
            },
        );
        m.insert(
            "kicad_sch",
            super::FileIcon {
                icon: '',
                color: "#ffffff",
            },
        );
        m.insert(
            "kicad_sym",
            super::FileIcon {
                icon: '',
                color: "#ffffff",
            },
        );
        m.insert(
            "kicad_wks",
            super::FileIcon {
                icon: '',
                color: "#ffffff",
            },
        );
        m.insert(
            "ko",
            super::FileIcon {
                icon: '',
                color: "#dcddd6",
            },
        );
        m.insert(
            "kpp",
            super::FileIcon {
                icon: '',
                color: "#f245fb",
            },
        );
        m.insert(
            "kra",
            super::FileIcon {
                icon: '',
                color: "#f245fb",
            },
        );
        m.insert(
            "krz",
            super::FileIcon {
                icon: '',
                color: "#f245fb",
            },
        );
        m.insert(
            "ksh",
            super::FileIcon {
                icon: '',
                color: "#4d5a5e",
            },
        );
        m.insert(
            "kt",
            super::FileIcon {
                icon: '',
                color: "#7F52FF",
            },
        );
        m.insert(
            "kts",
            super::FileIcon {
                icon: '',
                color: "#7F52FF",
            },
        );
        m.insert(
            "lck",
            super::FileIcon {
                icon: '',
                color: "#bbbbbb",
            },
        );
        m.insert(
            "leex",
            super::FileIcon {
                icon: '',
                color: "#a074c4",
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
                color: "#ECECEC",
            },
        );
        m.insert(
            "lhs",
            super::FileIcon {
                icon: '',
                color: "#a074c4",
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
                color: "#cbcb41",
            },
        );
        m.insert(
            "liquid",
            super::FileIcon {
                icon: '',
                color: "#95BF47",
            },
        );
        m.insert(
            "lock",
            super::FileIcon {
                icon: '',
                color: "#bbbbbb",
            },
        );
        m.insert(
            "log",
            super::FileIcon {
                icon: '󰌱',
                color: "#dddddd",
            },
        );
        m.insert(
            "lrc",
            super::FileIcon {
                icon: '󰨖',
                color: "#ffb713",
            },
        );
        m.insert(
            "lua",
            super::FileIcon {
                icon: '',
                color: "#51a0cf",
            },
        );
        m.insert(
            "luac",
            super::FileIcon {
                icon: '',
                color: "#51a0cf",
            },
        );
        m.insert(
            "luau",
            super::FileIcon {
                icon: '',
                color: "#00a2ff",
            },
        );
        m.insert(
            "m3u",
            super::FileIcon {
                icon: '󰲹',
                color: "#ed95ae",
            },
        );
        m.insert(
            "m3u8",
            super::FileIcon {
                icon: '󰲹',
                color: "#ed95ae",
            },
        );
        m.insert(
            "m4a",
            super::FileIcon {
                icon: '',
                color: "#00afff",
            },
        );
        m.insert(
            "m4v",
            super::FileIcon {
                icon: '',
                color: "#FD971F",
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
                color: "#6d8086",
            },
        );
        m.insert(
            "markdown",
            super::FileIcon {
                icon: '',
                color: "#dddddd",
            },
        );
        m.insert(
            "material",
            super::FileIcon {
                icon: '󰔉',
                color: "#B83998",
            },
        );
        m.insert(
            "md",
            super::FileIcon {
                icon: '',
                color: "#dddddd",
            },
        );
        m.insert(
            "md5",
            super::FileIcon {
                icon: '󰕥',
                color: "#8c86af",
            },
        );
        m.insert(
            "mdx",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "mint",
            super::FileIcon {
                icon: '󰌪',
                color: "#87c095",
            },
        );
        m.insert(
            "mjs",
            super::FileIcon {
                icon: '',
                color: "#f1e05a",
            },
        );
        m.insert(
            "mk",
            super::FileIcon {
                icon: '',
                color: "#6d8086",
            },
        );
        m.insert(
            "mkv",
            super::FileIcon {
                icon: '',
                color: "#FD971F",
            },
        );
        m.insert(
            "ml",
            super::FileIcon {
                icon: '',
                color: "#e37933",
            },
        );
        m.insert(
            "mli",
            super::FileIcon {
                icon: '',
                color: "#e37933",
            },
        );
        m.insert(
            "m",
            super::FileIcon {
                icon: '',
                color: "#599eff",
            },
        );
        m.insert(
            "mm",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "mo",
            super::FileIcon {
                icon: '∞',
                color: "#9772FB",
            },
        );
        m.insert(
            "mobi",
            super::FileIcon {
                icon: '',
                color: "#eab16d",
            },
        );
        m.insert(
            "mojo",
            super::FileIcon {
                icon: '',
                color: "#ff4c1f",
            },
        );
        m.insert(
            "🔥",
            super::FileIcon {
                icon: '',
                color: "#ff4c1f",
            },
        );
        m.insert(
            "mov",
            super::FileIcon {
                icon: '',
                color: "#FD971F",
            },
        );
        m.insert(
            "mp3",
            super::FileIcon {
                icon: '',
                color: "#00afff",
            },
        );
        m.insert(
            "mp4",
            super::FileIcon {
                icon: '',
                color: "#FD971F",
            },
        );
        m.insert(
            "mpp",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "msf",
            super::FileIcon {
                icon: '',
                color: "#137be1",
            },
        );
        m.insert(
            "mts",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "mustache",
            super::FileIcon {
                icon: '',
                color: "#e37933",
            },
        );
        m.insert(
            "nfo",
            super::FileIcon {
                icon: '',
                color: "#ffffcd",
            },
        );
        m.insert(
            "nim",
            super::FileIcon {
                icon: '',
                color: "#f3d400",
            },
        );
        m.insert(
            "nix",
            super::FileIcon {
                icon: '',
                color: "#7ebae4",
            },
        );
        m.insert(
            "nswag",
            super::FileIcon {
                icon: '',
                color: "#85ea2d",
            },
        );
        m.insert(
            "nu",
            super::FileIcon {
                icon: '>',
                color: "#3aa675",
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
                color: "#888888",
            },
        );
        m.insert(
            "ogg",
            super::FileIcon {
                icon: '',
                color: "#0075aa",
            },
        );
        m.insert(
            "opus",
            super::FileIcon {
                icon: '',
                color: "#0075aa",
            },
        );
        m.insert(
            "org",
            super::FileIcon {
                icon: '',
                color: "#77AA99",
            },
        );
        m.insert(
            "otf",
            super::FileIcon {
                icon: '',
                color: "#ECECEC",
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
                color: "#44cda8",
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
                color: "#6d8086",
            },
        );
        m.insert(
            "pcm",
            super::FileIcon {
                icon: '',
                color: "#0075aa",
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
                color: "#a074c4",
            },
        );
        m.insert(
            "pl",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "pls",
            super::FileIcon {
                icon: '󰲹',
                color: "#ed95ae",
            },
        );
        m.insert(
            "ply",
            super::FileIcon {
                icon: '󰆧',
                color: "#888888",
            },
        );
        m.insert(
            "pm",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "png",
            super::FileIcon {
                icon: '',
                color: "#a074c4",
            },
        );
        m.insert(
            "po",
            super::FileIcon {
                icon: '',
                color: "#2596be",
            },
        );
        m.insert(
            "pot",
            super::FileIcon {
                icon: '',
                color: "#2596be",
            },
        );
        m.insert(
            "pp",
            super::FileIcon {
                icon: '',
                color: "#FFA61A",
            },
        );
        m.insert(
            "ppt",
            super::FileIcon {
                icon: '󰈧',
                color: "#cb4a32",
            },
        );
        m.insert(
            "prisma",
            super::FileIcon {
                icon: '',
                color: "#5a67d8",
            },
        );
        m.insert(
            "pro",
            super::FileIcon {
                icon: '',
                color: "#e4b854",
            },
        );
        m.insert(
            "ps1",
            super::FileIcon {
                icon: '󰨊',
                color: "#4273ca",
            },
        );
        m.insert(
            "psd1",
            super::FileIcon {
                icon: '󰨊',
                color: "#6975c4",
            },
        );
        m.insert(
            "psm1",
            super::FileIcon {
                icon: '󰨊',
                color: "#6975c4",
            },
        );
        m.insert(
            "psb",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "psd",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "pub",
            super::FileIcon {
                icon: '󰷖',
                color: "#e3c58e",
            },
        );
        m.insert(
            "pxd",
            super::FileIcon {
                icon: '',
                color: "#5aa7e4",
            },
        );
        m.insert(
            "pxi",
            super::FileIcon {
                icon: '',
                color: "#5aa7e4",
            },
        );
        m.insert(
            "py",
            super::FileIcon {
                icon: '',
                color: "#ffbc03",
            },
        );
        m.insert(
            "pyc",
            super::FileIcon {
                icon: '',
                color: "#ffe291",
            },
        );
        m.insert(
            "pyd",
            super::FileIcon {
                icon: '',
                color: "#ffe291",
            },
        );
        m.insert(
            "pyi",
            super::FileIcon {
                icon: '',
                color: "#ffbc03",
            },
        );
        m.insert(
            "pyo",
            super::FileIcon {
                icon: '',
                color: "#ffe291",
            },
        );
        m.insert(
            "pyw",
            super::FileIcon {
                icon: '',
                color: "#5aa7e4",
            },
        );
        m.insert(
            "pyx",
            super::FileIcon {
                icon: '',
                color: "#5aa7e4",
            },
        );
        m.insert(
            "qm",
            super::FileIcon {
                icon: '',
                color: "#2596be",
            },
        );
        m.insert(
            "qml",
            super::FileIcon {
                icon: '',
                color: "#40cd52",
            },
        );
        m.insert(
            "qrc",
            super::FileIcon {
                icon: '',
                color: "#40cd52",
            },
        );
        m.insert(
            "qss",
            super::FileIcon {
                icon: '',
                color: "#40cd52",
            },
        );
        m.insert(
            "query",
            super::FileIcon {
                icon: '',
                color: "#90a850",
            },
        );
        m.insert(
            "r",
            super::FileIcon {
                icon: '󰟔',
                color: "#2266ba",
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
                color: "#eca517",
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
                color: "#cc3e44",
            },
        );
        m.insert(
            "resi",
            super::FileIcon {
                icon: '',
                color: "#f55385",
            },
        );
        m.insert(
            "rlib",
            super::FileIcon {
                icon: '',
                color: "#dea584",
            },
        );
        m.insert(
            "rmd",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "rproj",
            super::FileIcon {
                icon: '󰗆',
                color: "#358a5b",
            },
        );
        m.insert(
            "rs",
            super::FileIcon {
                icon: '',
                color: "#dea584",
            },
        );
        m.insert(
            "rss",
            super::FileIcon {
                icon: '',
                color: "#FB9D3B",
            },
        );
        m.insert(
            "sass",
            super::FileIcon {
                icon: '',
                color: "#f55385",
            },
        );
        m.insert(
            "sbt",
            super::FileIcon {
                icon: '',
                color: "#cc3e44",
            },
        );
        m.insert(
            "scad",
            super::FileIcon {
                icon: '',
                color: "#f9d72c",
            },
        );
        m.insert(
            "scala",
            super::FileIcon {
                icon: '',
                color: "#cc3e44",
            },
        );
        m.insert(
            "sc",
            super::FileIcon {
                icon: '',
                color: "#cc3e44",
            },
        );
        m.insert(
            "scm",
            super::FileIcon {
                icon: '󰘧',
                color: "#eeeeee",
            },
        );
        m.insert(
            "scss",
            super::FileIcon {
                icon: '',
                color: "#f55385",
            },
        );
        m.insert(
            "sh",
            super::FileIcon {
                icon: '',
                color: "#4d5a5e",
            },
        );
        m.insert(
            "sha1",
            super::FileIcon {
                icon: '󰕥',
                color: "#8c86af",
            },
        );
        m.insert(
            "sha224",
            super::FileIcon {
                icon: '󰕥',
                color: "#8c86af",
            },
        );
        m.insert(
            "sha256",
            super::FileIcon {
                icon: '󰕥',
                color: "#8c86af",
            },
        );
        m.insert(
            "sha384",
            super::FileIcon {
                icon: '󰕥',
                color: "#8c86af",
            },
        );
        m.insert(
            "sha512",
            super::FileIcon {
                icon: '󰕥',
                color: "#8c86af",
            },
        );
        m.insert(
            "sig",
            super::FileIcon {
                icon: 'λ',
                color: "#e37933",
            },
        );
        m.insert(
            "signature",
            super::FileIcon {
                icon: 'λ',
                color: "#e37933",
            },
        );
        m.insert(
            "skp",
            super::FileIcon {
                icon: '󰻫',
                color: "#839463",
            },
        );
        m.insert(
            "sldasm",
            super::FileIcon {
                icon: '󰻫',
                color: "#839463",
            },
        );
        m.insert(
            "sldprt",
            super::FileIcon {
                icon: '󰻫',
                color: "#839463",
            },
        );
        m.insert(
            "slim",
            super::FileIcon {
                icon: '',
                color: "#e34c26",
            },
        );
        m.insert(
            "sln",
            super::FileIcon {
                icon: '',
                color: "#854CC7",
            },
        );
        m.insert(
            "slvs",
            super::FileIcon {
                icon: '󰻫',
                color: "#839463",
            },
        );
        m.insert(
            "sml",
            super::FileIcon {
                icon: 'λ',
                color: "#e37933",
            },
        );
        m.insert(
            "so",
            super::FileIcon {
                icon: '',
                color: "#dcddd6",
            },
        );
        m.insert(
            "sol",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "spec.js",
            super::FileIcon {
                icon: '',
                color: "#cbcb41",
            },
        );
        m.insert(
            "spec.jsx",
            super::FileIcon {
                icon: '',
                color: "#20c2e3",
            },
        );
        m.insert(
            "spec.ts",
            super::FileIcon {
                icon: '',
                color: "#519aba",
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
                color: "#dad8d8",
            },
        );
        m.insert(
            "sqlite",
            super::FileIcon {
                icon: '',
                color: "#dad8d8",
            },
        );
        m.insert(
            "sqlite3",
            super::FileIcon {
                icon: '',
                color: "#dad8d8",
            },
        );
        m.insert(
            "srt",
            super::FileIcon {
                icon: '󰨖',
                color: "#ffb713",
            },
        );
        m.insert(
            "ssa",
            super::FileIcon {
                icon: '󰨖',
                color: "#ffb713",
            },
        );
        m.insert(
            "stl",
            super::FileIcon {
                icon: '󰆧',
                color: "#888888",
            },
        );
        m.insert(
            "strings",
            super::FileIcon {
                icon: '',
                color: "#2596be",
            },
        );
        m.insert(
            "ste",
            super::FileIcon {
                icon: '󰻫',
                color: "#839463",
            },
        );
        m.insert(
            "step",
            super::FileIcon {
                icon: '󰻫',
                color: "#839463",
            },
        );
        m.insert(
            "stp",
            super::FileIcon {
                icon: '󰻫',
                color: "#839463",
            },
        );
        m.insert(
            "styl",
            super::FileIcon {
                icon: '',
                color: "#8dc149",
            },
        );
        m.insert(
            "sub",
            super::FileIcon {
                icon: '󰨖',
                color: "#ffb713",
            },
        );
        m.insert(
            "sublime",
            super::FileIcon {
                icon: '',
                color: "#e37933",
            },
        );
        m.insert(
            "suo",
            super::FileIcon {
                icon: '',
                color: "#854CC7",
            },
        );
        m.insert(
            "sv",
            super::FileIcon {
                icon: '󰍛',
                color: "#019833",
            },
        );
        m.insert(
            "svelte",
            super::FileIcon {
                icon: '',
                color: "#ff3e00",
            },
        );
        m.insert(
            "svh",
            super::FileIcon {
                icon: '󰍛',
                color: "#019833",
            },
        );
        m.insert(
            "svg",
            super::FileIcon {
                icon: '󰜡',
                color: "#FFB13B",
            },
        );
        m.insert(
            "swift",
            super::FileIcon {
                icon: '',
                color: "#e37933",
            },
        );
        m.insert(
            "t",
            super::FileIcon {
                icon: '',
                color: "#519aba",
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
                color: "#dbbd30",
            },
        );
        m.insert(
            "terminal",
            super::FileIcon {
                icon: '',
                color: "#31B53E",
            },
        );
        m.insert(
            "test.js",
            super::FileIcon {
                icon: '',
                color: "#cbcb41",
            },
        );
        m.insert(
            "test.jsx",
            super::FileIcon {
                icon: '',
                color: "#20c2e3",
            },
        );
        m.insert(
            "test.ts",
            super::FileIcon {
                icon: '',
                color: "#519aba",
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
                color: "#5F43E9",
            },
        );
        m.insert(
            "tfvars",
            super::FileIcon {
                icon: '',
                color: "#5F43E9",
            },
        );
        m.insert(
            "tgz",
            super::FileIcon {
                icon: '',
                color: "#eca517",
            },
        );
        m.insert(
            "tmux",
            super::FileIcon {
                icon: '',
                color: "#14ba19",
            },
        );
        m.insert(
            "toml",
            super::FileIcon {
                icon: '',
                color: "#9c4221",
            },
        );
        m.insert(
            "torrent",
            super::FileIcon {
                icon: '',
                color: "#44cda8",
            },
        );
        m.insert(
            "tres",
            super::FileIcon {
                icon: '',
                color: "#6d8086",
            },
        );
        m.insert(
            "ts",
            super::FileIcon {
                icon: '',
                color: "#519aba",
            },
        );
        m.insert(
            "tscn",
            super::FileIcon {
                icon: '',
                color: "#6d8086",
            },
        );
        m.insert(
            "tsconfig",
            super::FileIcon {
                icon: '',
                color: "#FF8700",
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
                color: "#ECECEC",
            },
        );
        m.insert(
            "twig",
            super::FileIcon {
                icon: '',
                color: "#8dc149",
            },
        );
        m.insert(
            "txz",
            super::FileIcon {
                icon: '',
                color: "#eca517",
            },
        );
        m.insert(
            "typoscript",
            super::FileIcon {
                icon: '',
                color: "#FF8700",
            },
        );
        m.insert(
            "txt",
            super::FileIcon {
                icon: '󰈙',
                color: "#89e051",
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
                color: "#019833",
            },
        );
        m.insert(
            "vala",
            super::FileIcon {
                icon: '',
                color: "#7239b3",
            },
        );
        m.insert(
            "vh",
            super::FileIcon {
                icon: '󰍛',
                color: "#019833",
            },
        );
        m.insert(
            "vhd",
            super::FileIcon {
                icon: '󰍛',
                color: "#019833",
            },
        );
        m.insert(
            "vhdl",
            super::FileIcon {
                icon: '󰍛',
                color: "#019833",
            },
        );
        m.insert(
            "vim",
            super::FileIcon {
                icon: '',
                color: "#019833",
            },
        );
        m.insert(
            "vsh",
            super::FileIcon {
                icon: '',
                color: "#5d87bf",
            },
        );
        m.insert(
            "vsix",
            super::FileIcon {
                icon: '',
                color: "#854CC7",
            },
        );
        m.insert(
            "vue",
            super::FileIcon {
                icon: '',
                color: "#8dc149",
            },
        );
        m.insert(
            "wasm",
            super::FileIcon {
                icon: '',
                color: "#5c4cdb",
            },
        );
        m.insert(
            "wav",
            super::FileIcon {
                icon: '',
                color: "#00afff",
            },
        );
        m.insert(
            "webm",
            super::FileIcon {
                icon: '',
                color: "#FD971F",
            },
        );
        m.insert(
            "webmanifest",
            super::FileIcon {
                icon: '',
                color: "#f1e05a",
            },
        );
        m.insert(
            "webp",
            super::FileIcon {
                icon: '',
                color: "#a074c4",
            },
        );
        m.insert(
            "webpack",
            super::FileIcon {
                icon: '󰜫',
                color: "#519aba",
            },
        );
        m.insert(
            "wma",
            super::FileIcon {
                icon: '',
                color: "#00afff",
            },
        );
        m.insert(
            "woff",
            super::FileIcon {
                icon: '',
                color: "#ECECEC",
            },
        );
        m.insert(
            "woff2",
            super::FileIcon {
                icon: '',
                color: "#ECECEC",
            },
        );
        m.insert(
            "wrl",
            super::FileIcon {
                icon: '󰆧',
                color: "#888888",
            },
        );
        m.insert(
            "wrz",
            super::FileIcon {
                icon: '󰆧',
                color: "#888888",
            },
        );
        m.insert(
            "wv",
            super::FileIcon {
                icon: '',
                color: "#00afff",
            },
        );
        m.insert(
            "wvc",
            super::FileIcon {
                icon: '',
                color: "#00afff",
            },
        );
        m.insert(
            "x",
            super::FileIcon {
                icon: '',
                color: "#599eff",
            },
        );
        m.insert(
            "xm",
            super::FileIcon {
                icon: '',
                color: "#519aba",
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
                color: "#635b46",
            },
        );
        m.insert(
            "xcplayground",
            super::FileIcon {
                icon: '',
                color: "#e37933",
            },
        );
        m.insert(
            "xcstrings",
            super::FileIcon {
                icon: '',
                color: "#2596be",
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
                color: "#e37933",
            },
        );
        m.insert(
            "xpi",
            super::FileIcon {
                icon: '',
                color: "#ff1b01",
            },
        );
        m.insert(
            "xul",
            super::FileIcon {
                icon: '',
                color: "#e37933",
            },
        );
        m.insert(
            "xz",
            super::FileIcon {
                icon: '',
                color: "#eca517",
            },
        );
        m.insert(
            "yaml",
            super::FileIcon {
                icon: '',
                color: "#6d8086",
            },
        );
        m.insert(
            "yml",
            super::FileIcon {
                icon: '',
                color: "#6d8086",
            },
        );
        m.insert(
            "zig",
            super::FileIcon {
                icon: '',
                color: "#f69a1b",
            },
        );
        m.insert(
            "zip",
            super::FileIcon {
                icon: '',
                color: "#eca517",
            },
        );
        m.insert(
            "zsh",
            super::FileIcon {
                icon: '',
                color: "#89e051",
            },
        );
        m.insert(
            "zst",
            super::FileIcon {
                icon: '',
                color: "#eca517",
            },
        );
        m
    };
}
