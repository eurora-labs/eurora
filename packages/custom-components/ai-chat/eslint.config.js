import { config } from '@eurora/eslint-config/index.js';

export default [
	...config,
	{
		ignores: ['.svelte-kit/*'],
	},
];
