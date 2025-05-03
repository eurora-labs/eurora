import type { Config } from 'tailwindcss';
import presets from '@eurora/ui/tailwind.config';

const config: Config = {
	content: [
		'./src/**/*.{html,js,svelte,ts}',
		'../../packages/ui/src/**/*.{html,js,svelte,ts}',
		'../../packages/custom-components/ai-chat/src/**/*.{html,js,svelte,ts}',
		'../../packages/custom-components/main-sidebar/src/**/*.{html,js,svelte,ts}',
		'../../packages/custom-components/launcher/src/**/*.{html,js,svelte,ts}'
	],
	presets: [presets]
};

export default config;
