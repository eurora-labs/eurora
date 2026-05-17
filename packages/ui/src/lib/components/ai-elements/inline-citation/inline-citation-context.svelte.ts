import { getContext, setContext } from 'svelte';

const CAROUSEL_CONTEXT_KEY = Symbol.for('inline-citation-carousel');

export interface CarouselStateOptions {
	total: () => number;
}

export class CarouselState {
	readonly #opts: CarouselStateOptions;
	#currentIndex = $state(0);

	constructor(opts: CarouselStateOptions) {
		this.#opts = opts;
	}

	get currentIndex() {
		return this.#currentIndex;
	}

	set currentIndex(value: number) {
		this.#currentIndex = value;
	}

	get total(): number {
		return this.#opts.total();
	}

	next() {
		if (this.#currentIndex < this.total - 1) {
			this.#currentIndex++;
		}
	}

	prev() {
		if (this.#currentIndex > 0) {
			this.#currentIndex--;
		}
	}
}

export function setCarouselContext(state: CarouselState) {
	setContext(CAROUSEL_CONTEXT_KEY, state);
}

export function getCarouselContext(): CarouselState {
	const context = getContext<CarouselState | undefined>(CAROUSEL_CONTEXT_KEY);
	if (!context) {
		throw new Error(
			'InlineCitationCarousel components must be used within InlineCitationCarousel',
		);
	}
	return context;
}
