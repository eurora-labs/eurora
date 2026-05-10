import type { LanguageRegistration } from 'shiki/core';

export interface BundledLanguage {
	id: string;
	aliases?: string[];
	import: () => Promise<{ default: LanguageRegistration | LanguageRegistration[] }>;
}

/**
 * Languages available to the streaming highlighter. Mirrors the set bundled
 * by `svelte-streamdown` (which renders the post-stream view), so that code
 * blocks display the same colour palette during streaming and after the
 * stream settles.
 */
export const bundledLanguages: BundledLanguage[] = [
	// Web essentials
	{ id: 'javascript', aliases: ['js'], import: () => import('@shikijs/langs/javascript') },
	{ id: 'typescript', aliases: ['ts'], import: () => import('@shikijs/langs/typescript') },
	{ id: 'html', import: () => import('@shikijs/langs/html') },
	{ id: 'css', import: () => import('@shikijs/langs/css') },
	{ id: 'json', import: () => import('@shikijs/langs/json') },
	{ id: 'jsx', import: () => import('@shikijs/langs/jsx') },
	{ id: 'tsx', import: () => import('@shikijs/langs/tsx') },
	{ id: 'markdown', aliases: ['md'], import: () => import('@shikijs/langs/markdown') },
	{ id: 'yaml', aliases: ['yml'], import: () => import('@shikijs/langs/yaml') },
	{ id: 'xml', import: () => import('@shikijs/langs/xml') },

	// Backend languages
	{ id: 'python', aliases: ['py'], import: () => import('@shikijs/langs/python') },
	{ id: 'java', import: () => import('@shikijs/langs/java') },
	{ id: 'go', import: () => import('@shikijs/langs/go') },
	{ id: 'rust', aliases: ['rs'], import: () => import('@shikijs/langs/rust') },
	{ id: 'ruby', aliases: ['rb'], import: () => import('@shikijs/langs/ruby') },
	{ id: 'php', import: () => import('@shikijs/langs/php') },
	{ id: 'c', import: () => import('@shikijs/langs/c') },
	{ id: 'cpp', aliases: ['c++'], import: () => import('@shikijs/langs/cpp') },
	{ id: 'csharp', aliases: ['c#', 'cs'], import: () => import('@shikijs/langs/csharp') },
	{ id: 'sql', import: () => import('@shikijs/langs/sql') },
	{ id: 'swift', import: () => import('@shikijs/langs/swift') },
	{ id: 'kotlin', aliases: ['kt', 'kts'], import: () => import('@shikijs/langs/kotlin') },

	// Config / shell
	{
		id: 'shellscript',
		aliases: ['bash', 'sh', 'shell', 'zsh'],
		import: () => import('@shikijs/langs/shellscript'),
	},
	{ id: 'docker', aliases: ['dockerfile'], import: () => import('@shikijs/langs/docker') },
	{ id: 'toml', import: () => import('@shikijs/langs/toml') },
	{ id: 'graphql', aliases: ['gql'], import: () => import('@shikijs/langs/graphql') },
	{ id: 'svelte', import: () => import('@shikijs/langs/svelte') },
	{ id: 'vue', import: () => import('@shikijs/langs/vue') },
];

const supported = new Set<string>();
for (const lang of bundledLanguages) {
	supported.add(lang.id);
	if (lang.aliases) for (const alias of lang.aliases) supported.add(alias);
}

export function isLanguageSupported(lang: string | undefined | null): boolean {
	return !!lang && supported.has(lang);
}
