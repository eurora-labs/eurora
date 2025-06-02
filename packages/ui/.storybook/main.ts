import type { StorybookConfig } from '@storybook/sveltekit';

const config: StorybookConfig = {
	stories: ['../src/**/*.stories.@(js|ts|svelte)'],
	addons: [
		'@storybook/addon-links',
		'@storybook/addon-essentials',
		'@storybook/addon-docs',
		'@storybook/addon-svelte-csf',
		'storybook-dark-mode'
	],
	framework: {
		name: '@storybook/sveltekit',
		options: {}
	},
	typescript: {
		check: false
	},
	staticDirs: ['../static']
};

export default config;
