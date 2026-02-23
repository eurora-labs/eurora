import { getContext, setContext } from 'svelte';

export type ToolUIPartApproval =
	| {
			id: string;
			approved?: never;
			reason?: never;
		}
	| {
			id: string;
			approved: boolean;
			reason?: string;
		}
	| {
			id: string;
			approved: true;
			reason?: string;
		}
	| {
			id: string;
			approved: false;
			reason?: string;
		}
	| undefined;

export type ToolUIPartState =
	| 'approval-requested'
	| 'approval-responded'
	| 'input-streaming'
	| 'input-available'
	| 'output-available'
	| 'output-denied'
	| 'output-error';

type Getter<T> = () => T;

export type ConfirmationStateProps = {
	approval: Getter<ToolUIPartApproval>;
	state: Getter<ToolUIPartState>;
};

class ConfirmationState {
	readonly props: ConfirmationStateProps;
	approval = $derived.by(() => this.props.approval());
	state = $derived.by(() => this.props.state());

	constructor(props: ConfirmationStateProps) {
		this.props = props;
	}
}

const SYMBOL_KEY = 'ai-confirmation';

export function setConfirmation(props: ConfirmationStateProps): ConfirmationState {
	return setContext(Symbol.for(SYMBOL_KEY), new ConfirmationState(props));
}

export function useConfirmation(): ConfirmationState {
	return getContext(Symbol.for(SYMBOL_KEY));
}
