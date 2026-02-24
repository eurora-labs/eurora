import { getContext, setContext } from 'svelte';

export type ChangeType = 'major' | 'minor' | 'patch' | 'added' | 'removed';

type Getter<T> = () => T;

export type PackageInfoStateProps = {
	name: Getter<string>;
	currentVersion: Getter<string | undefined>;
	newVersion: Getter<string | undefined>;
	changeType: Getter<ChangeType | undefined>;
};

class PackageInfoState {
	readonly props: PackageInfoStateProps;
	name = $derived.by(() => this.props.name());
	currentVersion = $derived.by(() => this.props.currentVersion());
	newVersion = $derived.by(() => this.props.newVersion());
	changeType = $derived.by(() => this.props.changeType());

	constructor(props: PackageInfoStateProps) {
		this.props = props;
	}
}

const SYMBOL_KEY = 'ai-package-info';

export function setPackageInfo(props: PackageInfoStateProps): PackageInfoState {
	return setContext(Symbol.for(SYMBOL_KEY), new PackageInfoState(props));
}

export function usePackageInfo(): PackageInfoState {
	return getContext(Symbol.for(SYMBOL_KEY));
}
