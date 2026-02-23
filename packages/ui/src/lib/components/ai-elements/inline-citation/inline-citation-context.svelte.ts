import { getContext, setContext } from 'svelte';

const CAROUSEL_CONTEXT_KEY = Symbol.for('inline-citation-carousel');

export class CarouselState {
	#currentIndex = $state(0);
	#total = $state(0);

	constructor(options: { currentIndex?: number; total?: number }) {
		this.#currentIndex = options.currentIndex ?? 0;
		this.#total = options.total ?? 0;
	}

	get currentIndex() {
		return this.#currentIndex;
	}

	set currentIndex(value: number) {
		this.#currentIndex = value;
	}

	get total() {
		return this.#total;
	}

	set total(value: number) {
		this.#total = value;
	}

	next() {
		if (this.#currentIndex < this.#total - 1) {
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
