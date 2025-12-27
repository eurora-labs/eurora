export * from './typings/index.js';
export * from './SvelteNodeView.js';

import { default as Editor } from '$lib/Editor.svelte';

export { Editor };

export { TextSelection } from 'prosemirror-state';
