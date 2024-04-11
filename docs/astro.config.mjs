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
      defaultLocale: 'root',
      locales: {
        // English docs in `src/content/docs/en/`
        root: {
          label: 'English',
          lang: 'en',
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
          translations: {
            'zh-CN': '信息',
          },
          items: [
            {
              label: 'Introduction',
              translations: {
                'zh-CN': '介绍',
              },
              link: '/',
            },
          ],
        },
        {
          label: 'Getting Started',
          translations: {
            'zh-CN': '快速开始',
          },
          autogenerate: { directory: 'getting-started' },
        },
        {
          label: 'Features',
          translations: {
            'zh-CN': '特性',
          },
          autogenerate: { directory: 'features' },
        },
        {
          label: 'API Reference',
          translations: {
            'zh-CN': 'API 参考',
          },
          link: 'api/',
        },
      ],
    }),
  ],
})
