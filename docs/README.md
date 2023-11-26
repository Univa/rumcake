# rumcake user docs

[![Built with Starlight](https://astro.badg.es/v2/built-with-starlight/tiny.svg)](https://starlight.astro.build)

The following documentation was adapted from the Starlight template:

```
npm create astro@latest -- --template starlight
```

## Requirements

This doc site is built using Astro's Starlight integration.

To develop documentation, make sure you have:

- Node.js (version 18+ should be fine)
- [`yarn`](https://yarnpkg.com/)

> [!NOTE]
> This project uses `yarn` instead of `npm` for package management.

## Project Structure

```
.
├── public/
├── src/
│   ├── assets/
│   ├── content/
│   │   ├── docs/
│   │   └── config.ts
│   └── env.d.ts
├── astro.config.mjs
├── package.json
└── tsconfig.json
```

Starlight looks for `.md` or `.mdx` files in the `src/content/docs/` directory. Each file is exposed as a route based on its file name.

Images can be added to `src/assets/` and embedded in Markdown with a relative link.

Static assets, like favicons, can be placed in the `public/` directory.

## Commands

All commands are run from the root of the project, from a terminal:

| Command                  | Action                                           |
| :----------------------- | :----------------------------------------------- |
| `yarn install`           | Installs dependencies                            |
| `yarn dev`               | Starts local dev server at `localhost:4321`      |
| `yarn build`             | Build your production site to `./dist/`          |
| `yarn preview`           | Preview your build locally, before deploying     |
| `yarn exec astro ...`    | Run CLI commands like `astro add`, `astro check` |
| `yarn exec astro --help` | Get help using the Astro CLI                     |

## Want to learn more?

Check out [Starlight’s docs](https://starlight.astro.build/), read [the Astro documentation](https://docs.astro.build), or jump into the [Astro Discord server](https://astro.build/chat).
