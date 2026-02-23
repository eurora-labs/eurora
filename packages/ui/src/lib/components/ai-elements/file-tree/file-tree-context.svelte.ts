import { getContext, setContext } from 'svelte';

type Getter<T> = () => T;

export type FileTreeStateProps = {
	expandedPaths: Getter<Set<string>>;
	togglePath: (path: string) => void;
	selectedPath: Getter<string | undefined>;
	onSelect: Getter<((path: string) => void) | undefined>;
};

class FileTreeState {
	readonly props: FileTreeStateProps;
	expandedPaths = $derived.by(() => this.props.expandedPaths());
	selectedPath = $derived.by(() => this.props.selectedPath());

	constructor(props: FileTreeStateProps) {
		this.props = props;
	}

	togglePath(path: string) {
		this.props.togglePath(path);
	}

	select(path: string) {
		this.props.onSelect()?.(path);
	}
}

const SYMBOL_KEY = 'ai-file-tree';

export function setFileTree(props: FileTreeStateProps): FileTreeState {
	return setContext(Symbol.for(SYMBOL_KEY), new FileTreeState(props));
}

export function useFileTree(): FileTreeState {
	return getContext(Symbol.for(SYMBOL_KEY));
}
