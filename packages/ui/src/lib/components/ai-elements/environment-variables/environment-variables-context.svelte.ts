import { getContext, setContext } from 'svelte';

type Getter<T> = () => T;

export type EnvironmentVariablesStateProps = {
	showValues: Getter<boolean>;
	setShowValues: (show: boolean) => void;
};

class EnvironmentVariablesState {
	readonly props: EnvironmentVariablesStateProps;
	showValues = $derived.by(() => this.props.showValues());

	constructor(props: EnvironmentVariablesStateProps) {
		this.props = props;
	}

	setShowValues(show: boolean) {
		this.props.setShowValues(show);
	}
}

const SYMBOL_KEY = 'ai-environment-variables';

export function setEnvironmentVariables(
	props: EnvironmentVariablesStateProps,
): EnvironmentVariablesState {
	return setContext(Symbol.for(SYMBOL_KEY), new EnvironmentVariablesState(props));
}

export function useEnvironmentVariables(): EnvironmentVariablesState {
	return getContext(Symbol.for(SYMBOL_KEY));
}

export type EnvironmentVariableStateProps = {
	name: Getter<string>;
	value: Getter<string>;
};

class EnvironmentVariableState {
	readonly props: EnvironmentVariableStateProps;
	name = $derived.by(() => this.props.name());
	value = $derived.by(() => this.props.value());

	constructor(props: EnvironmentVariableStateProps) {
		this.props = props;
	}
}

const VARIABLE_SYMBOL_KEY = 'ai-environment-variable';

export function setEnvironmentVariable(
	props: EnvironmentVariableStateProps,
): EnvironmentVariableState {
	return setContext(Symbol.for(VARIABLE_SYMBOL_KEY), new EnvironmentVariableState(props));
}

export function useEnvironmentVariable(): EnvironmentVariableState {
	return getContext(Symbol.for(VARIABLE_SYMBOL_KEY));
}
