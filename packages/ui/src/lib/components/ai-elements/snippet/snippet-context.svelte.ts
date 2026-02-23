import { getContext, setContext } from 'svelte';

type Getter<T> = () => T;

export type SnippetStateProps = {
	code: Getter<string>;
};

class SnippetState {
	readonly props: SnippetStateProps;
	code = $derived.by(() => this.props.code());

	constructor(props: SnippetStateProps) {
		this.props = props;
	}
}

const SYMBOL_KEY = 'ai-snippet';

export function setSnippet(props: SnippetStateProps): SnippetState {
	return setContext(Symbol.for(SYMBOL_KEY), new SnippetState(props));
}

export function useSnippet(): SnippetState {
	return getContext(Symbol.for(SYMBOL_KEY));
}
