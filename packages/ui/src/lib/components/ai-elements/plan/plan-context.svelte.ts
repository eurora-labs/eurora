import { getContext, setContext } from 'svelte';

type Getter<T> = () => T;

export type PlanStateProps = {
	isStreaming: Getter<boolean>;
};

class PlanState {
	readonly props: PlanStateProps;
	isStreaming = $derived.by(() => this.props.isStreaming());

	constructor(props: PlanStateProps) {
		this.props = props;
	}
}

const SYMBOL_KEY = 'ai-plan';

export function setPlan(props: PlanStateProps): PlanState {
	return setContext(Symbol.for(SYMBOL_KEY), new PlanState(props));
}

export function usePlan(): PlanState {
	return getContext(Symbol.for(SYMBOL_KEY));
}
