import starlight from '@astrojs/starlight'
import { defineConfig } from 'astro/config'

// https://astro.build/config
export default defineConfig({
  site: 'https://univa.github.io',
  base: '/rumcake',
  trailingSlash: 'always',
  build: {
    format: 'directory',
  },
  integrations: [
    starlight({
      title: 'rumcake',
      defaultLocale: 'en',
      locales: {
        // English docs in `src/content/docs/en/`
        en: {
          label: 'English',
        },
        // Simplified Chinese docs in `src/content/docs/zh-cn/`
        'zh-cn': {
          label: '简体中文',
          lang: 'zh-CN',
        },
      },
      lastUpdated: true,
      tableOfContents: {
        minHeadingLevel: 1,
      },
      social: {
        github: 'https://github.com/Univa/rumcake',
      },
      pagination: false,
      sidebar: [
        {
          label: 'Information',
          items: [
            {
              label: 'Introduction',
              link: '/',
            },
          ],
        },
        {
          label: 'Getting Started',
          autogenerate: { directory: 'getting-started' },
        },
        {
          label: 'Features',
          autogenerate: { directory: 'features' },
        },
        {
          label: 'API Reference',
          link: 'api/',
        },
      ],
    }),
  ],
})
