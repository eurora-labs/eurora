import { getContext, setContext } from 'svelte';

const PERSONA_CONTEXT_KEY = Symbol.for('persona-context');

export type PersonaVariant = 'command' | 'glint' | 'halo' | 'mana' | 'obsidian' | 'opal';
export type PersonaState = 'idle' | 'listening' | 'thinking' | 'speaking' | 'asleep';

export const sources: Record<
	PersonaVariant,
	{ source: string; dynamicColor: boolean; hasModel: boolean }
> = {
	command: {
		dynamicColor: true,
		hasModel: true,
		source: 'https://ejiidnob33g9ap1r.public.blob.vercel-storage.com/command-2.0.riv',
	},
	glint: {
		dynamicColor: true,
		hasModel: true,
		source: 'https://ejiidnob33g9ap1r.public.blob.vercel-storage.com/glint-2.0.riv',
	},
	halo: {
		dynamicColor: true,
		hasModel: true,
		source: 'https://ejiidnob33g9ap1r.public.blob.vercel-storage.com/halo-2.0.riv',
	},
	mana: {
		dynamicColor: false,
		hasModel: true,
		source: 'https://ejiidnob33g9ap1r.public.blob.vercel-storage.com/mana-2.0.riv',
	},
	obsidian: {
		dynamicColor: true,
		hasModel: true,
		source: 'https://ejiidnob33g9ap1r.public.blob.vercel-storage.com/obsidian-2.0.riv',
	},
	opal: {
		dynamicColor: false,
		hasModel: false,
		source: 'https://ejiidnob33g9ap1r.public.blob.vercel-storage.com/orb-1.2.riv',
	},
};

export class PersonaContext {
	#variant = $state<PersonaVariant>('obsidian');
	#state = $state<PersonaState>('idle');
	#colorRgb = $state<[number, number, number]>([0, 0, 0]);

	constructor(
		options: {
			variant?: PersonaVariant;
			state?: PersonaState;
		} = {},
	) {
		this.#variant = options.variant ?? 'obsidian';
		this.#state = options.state ?? 'idle';
	}

	get variant() {
		return this.#variant;
	}

	set variant(value: PersonaVariant) {
		this.#variant = value;
	}

	get state() {
		return this.#state;
	}

	set state(value: PersonaState) {
		this.#state = value;
	}

	get colorRgb() {
		return this.#colorRgb;
	}

	set colorRgb(value: [number, number, number]) {
		this.#colorRgb = value;
	}

	get source() {
		return sources[this.#variant];
	}
}

export function setPersonaContext(context: PersonaContext) {
	setContext(PERSONA_CONTEXT_KEY, context);
}

export function getPersonaContext(): PersonaContext {
	const context = getContext<PersonaContext | undefined>(PERSONA_CONTEXT_KEY);
	if (!context) {
		throw new Error('Persona components must be used within Persona');
	}
	return context;
}
