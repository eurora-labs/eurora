import type { Preview } from '@storybook/svelte';
import '../src/styles/main.css';

document.body.classList.add('dark');
document.documentElement.classList.add('dark');

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
