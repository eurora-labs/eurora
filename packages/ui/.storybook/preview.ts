import type { Preview } from '@storybook/svelte';
import '../src/styles/main.pcss';

const preview: Preview = {
	parameters: {
		controls: {
			matchers: {
				color: /(background|color)$/i,
				date: /Date$/i,
			},
		},
		docs: {
			extractComponentDescription: (component, { notes }) => {
				if (notes) {
					return typeof notes === 'string' ? notes : notes.markdown || notes.text;
				}
				return null;
			},
		},
	},
};

export default preview;
