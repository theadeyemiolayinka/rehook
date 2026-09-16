import { defineConfig } from 'vitepress';

export default defineConfig({
  title: 'Rehook',
  description: 'Self-hosted webhook relay for local development: capture, inspect, and replay webhooks to configured local targets',
  lang: 'en',
  cleanUrls: true,
  lastUpdated: true,
  // Served at the apex of the custom domain (rehook.bytao.dev) via GitHub
  // Pages. If the custom domain is ever removed, this must become
  // '/rehook/' for the default github.io project path to work.
  base: '/',
  sitemap: {
    hostname: 'https://rehook.bytao.dev',
  },

  head: [
    ['link', { rel: 'icon', type: 'image/svg+xml', href: '/logo.svg' }],
    ['meta', { name: 'robots', content: 'index, follow' }],
  ],

  themeConfig: {
    logo: '/logo.svg',

    nav: [
      { text: 'Docs', link: '/getting-started/server' },
      { text: 'GitHub', link: 'https://github.com/theadeyemiolayinka/rehook' },
    ],

    sidebar: [
      {
        text: 'Getting Started',
        items: [
          { text: 'Server Setup', link: '/getting-started/server' },
          { text: 'Agent Setup', link: '/getting-started/agent' },
        ],
      },
      {
        text: 'Server',
        items: [
          { text: 'Configuration', link: '/server/configuration' },
        ],
      },
      {
        text: 'Agent',
        items: [
          { text: 'Installation', link: '/agent/installation' },
          { text: 'Targets', link: '/agent/targets' },
          { text: 'Web UI', link: '/agent/web-ui' },
        ],
      },
      {
        text: 'Architecture',
        items: [
          { text: 'Overview', link: '/architecture/overview' },
          { text: 'Security Model', link: '/architecture/security-model' },
        ],
      },
      {
        text: 'Deployment',
        items: [
          { text: 'Docker', link: '/deployment/docker' },
          { text: 'Coolify/Dokploy', link: '/deployment/coolify' },
          { text: 'Production', link: '/deployment/production' },
        ],
      },
      {
        text: 'Security',
        items: [
          { text: 'Reporting', link: '/security/reporting' },
        ],
      },
      {
        text: 'Development',
        items: [
          { text: 'Setup', link: '/development/setup' },
          { text: 'Testing', link: '/development/testing' },
        ],
      },
    ],

    socialLinks: [
      { icon: 'github', link: 'https://github.com/theadeyemiolayinka/rehook' },
    ],

    footer: {
      message: 'Released under the MIT or Apache-2.0 License.',
      copyright: 'Copyright 2024-2026 TheAdeyemiOlayinka <github.com/theadeyemiolayinka>',
    },

    outline: {
      level: [2, 3],
    },

    search: {
      provider: 'local',
    },
  },
});
