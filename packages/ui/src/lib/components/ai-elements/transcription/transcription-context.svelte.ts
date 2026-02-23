import { getContext, setContext } from 'svelte';

export interface TranscriptionSegment {
	text: string;
	startSecond: number;
	endSecond: number;
}

type Getter<T> = () => T;

export type TranscriptionStateProps = {
	currentTime: Getter<number>;
	onSeek: Getter<((time: number) => void) | undefined>;
};

class TranscriptionState {
	readonly props: TranscriptionStateProps;
	currentTime = $derived.by(() => this.props.currentTime());
	onSeek = $derived.by(() => this.props.onSeek());

	constructor(props: TranscriptionStateProps) {
		this.props = props;
	}
}

const SYMBOL_KEY = 'ai-transcription';

export function setTranscription(props: TranscriptionStateProps): TranscriptionState {
	return setContext(Symbol.for(SYMBOL_KEY), new TranscriptionState(props));
}

export function useTranscription(): TranscriptionState {
	return getContext(Symbol.for(SYMBOL_KEY));
}
