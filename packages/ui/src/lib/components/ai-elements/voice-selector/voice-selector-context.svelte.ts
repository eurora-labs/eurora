import { getContext, setContext } from 'svelte';

const VOICE_SELECTOR_CONTEXT_KEY = Symbol.for('voice-selector-context');

export class VoiceSelectorContext {
	#value = $state<string | undefined>(undefined);
	#open = $state(false);
	#onValueChange: ((value: string | undefined) => void) | undefined;
	#onOpenChange: ((open: boolean) => void) | undefined;

	constructor(
		options: {
			value?: string;
			open?: boolean;
			onValueChange?: (value: string | undefined) => void;
			onOpenChange?: (open: boolean) => void;
		} = {},
	) {
		this.#value = options.value;
		this.#open = options.open ?? false;
		this.#onValueChange = options.onValueChange;
		this.#onOpenChange = options.onOpenChange;
	}

	get value() {
		return this.#value;
	}

	set value(val: string | undefined) {
		this.#value = val;
		this.#onValueChange?.(val);
	}

	get open() {
		return this.#open;
	}

	set open(val: boolean) {
		this.#open = val;
		this.#onOpenChange?.(val);
	}
}

export function setVoiceSelectorContext(context: VoiceSelectorContext) {
	setContext(VOICE_SELECTOR_CONTEXT_KEY, context);
}

export function getVoiceSelectorContext(): VoiceSelectorContext {
	const context = getContext<VoiceSelectorContext | undefined>(VOICE_SELECTOR_CONTEXT_KEY);
	if (!context) {
		throw new Error('VoiceSelector components must be used within VoiceSelector');
	}
	return context;
}

export type GenderValue =
	| 'male'
	| 'female'
	| 'transgender'
	| 'androgyne'
	| 'non-binary'
	| 'intersex';

export type AccentValue =
	| 'american'
	| 'british'
	| 'australian'
	| 'canadian'
	| 'irish'
	| 'scottish'
	| 'indian'
	| 'south-african'
	| 'new-zealand'
	| 'spanish'
	| 'french'
	| 'german'
	| 'italian'
	| 'portuguese'
	| 'brazilian'
	| 'mexican'
	| 'argentinian'
	| 'japanese'
	| 'chinese'
	| 'korean'
	| 'russian'
	| 'arabic'
	| 'dutch'
	| 'swedish'
	| 'norwegian'
	| 'danish'
	| 'finnish'
	| 'polish'
	| 'turkish'
	| 'greek'
	| (string & {});

export const accentEmojiMap: Record<string, string> = {
	american: '\u{1F1FA}\u{1F1F8}',
	british: '\u{1F1EC}\u{1F1E7}',
	australian: '\u{1F1E6}\u{1F1FA}',
	canadian: '\u{1F1E8}\u{1F1E6}',
	irish: '\u{1F1EE}\u{1F1EA}',
	scottish: '\u{1F3F4}\u{E0067}\u{E0062}\u{E0073}\u{E0063}\u{E0074}\u{E007F}',
	indian: '\u{1F1EE}\u{1F1F3}',
	'south-african': '\u{1F1FF}\u{1F1E6}',
	'new-zealand': '\u{1F1F3}\u{1F1FF}',
	spanish: '\u{1F1EA}\u{1F1F8}',
	french: '\u{1F1EB}\u{1F1F7}',
	german: '\u{1F1E9}\u{1F1EA}',
	italian: '\u{1F1EE}\u{1F1F9}',
	portuguese: '\u{1F1F5}\u{1F1F9}',
	brazilian: '\u{1F1E7}\u{1F1F7}',
	mexican: '\u{1F1F2}\u{1F1FD}',
	argentinian: '\u{1F1E6}\u{1F1F7}',
	japanese: '\u{1F1EF}\u{1F1F5}',
	chinese: '\u{1F1E8}\u{1F1F3}',
	korean: '\u{1F1F0}\u{1F1F7}',
	russian: '\u{1F1F7}\u{1F1FA}',
	arabic: '\u{1F1F8}\u{1F1E6}',
	dutch: '\u{1F1F3}\u{1F1F1}',
	swedish: '\u{1F1F8}\u{1F1EA}',
	norwegian: '\u{1F1F3}\u{1F1F4}',
	danish: '\u{1F1E9}\u{1F1F0}',
	finnish: '\u{1F1EB}\u{1F1EE}',
	polish: '\u{1F1F5}\u{1F1F1}',
	turkish: '\u{1F1F9}\u{1F1F7}',
	greek: '\u{1F1EC}\u{1F1F7}',
};
