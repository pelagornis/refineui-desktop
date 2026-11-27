// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import pagePlugin from '@pelagornis/page';

export default defineConfig({
  site: 'https://pelagornis.github.io',
  base: process.env.DOCS_BASE ?? '/',
  integrations: [
    starlight({
      title: 'RefineUI Desktop',
      description: 'RefineUI Desktop — a desktop-native design system for GPUI apps.',
      plugins: [
        pagePlugin({
          footerText: '© 2026 Pelagornis',
          navigation: [
            { href: '/getting-started/installation/', label: 'Getting started' },
            { sidebar: 'Architecture' },
            { sidebar: 'Foundations' },
            { sidebar: 'Components' },
          ],
        }),
      ],
      social: [
        { icon: 'github', label: 'GitHub', href: 'https://github.com/pelagornis/refineui-desktop' },
      ],
      customCss: ['./src/styles/global.css'],
      sidebar: [
        { label: 'Getting started', items: [{ autogenerate: { directory: 'getting-started' } }] },
        { label: 'Architecture', items: [{ autogenerate: { directory: 'architecture' } }] },
        { label: 'Foundations', items: [{ autogenerate: { directory: 'foundations' } }] },
        { label: 'Components', items: [{ autogenerate: { directory: 'components' } }] },
        { label: 'Desktop shell', items: [{ autogenerate: { directory: 'shell' } }] },
        { label: 'Workspace', items: [{ autogenerate: { directory: 'workspace' } }] },
        { label: 'Guides', items: [{ autogenerate: { directory: 'guides' } }] },
        { label: 'API reference', items: [{ autogenerate: { directory: 'reference' } }] },
        { label: 'Final report', items: [{ autogenerate: { directory: 'report' } }] },
      ],
    }),
  ],
});
