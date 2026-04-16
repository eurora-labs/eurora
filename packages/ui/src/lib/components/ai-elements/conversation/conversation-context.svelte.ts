import { watch } from 'runed';
import { getContext, hasContext, setContext } from 'svelte';

const STICK_TO_BOTTOM_CONTEXT_KEY = Symbol.for('stick-to-bottom-context');
const TOUCH_DEAD_ZONE_PX = 10;
const RE_ENGAGE_THRESHOLD_PX = 20;

export class StickToBottomContext {
	#element: HTMLElement | null = $state(null);
	#userScrolledAway = $state(false);
	#resizeObserver: ResizeObserver | null = null;
	#mutationObserver: MutationObserver | null = null;
	#rafScheduled = false;
	#touchStartY: number | null = null;

	isAtBottom = $derived(!this.#userScrolledAway);

	constructor() {
		watch(
			() => this.#element,
			() => {
				if (this.#element) {
					this.#setupObservers();
					return () => this.#cleanup();
				}
			},
		);
	}

	setElement(element: HTMLElement) {
		this.#element = element;
	}

	scrollToBottom = (behavior: ScrollBehavior = 'smooth') => {
		if (!this.#element) return;
		this.#userScrolledAway = false;
		this.#element.scrollTo({
			top: this.#element.scrollHeight,
			behavior,
		});
	};

	reengageAutoScroll = () => {
		this.#userScrolledAway = false;
		if (this.#element) {
			this.#element.scrollTo({ top: this.#element.scrollHeight, behavior: 'auto' });
		}
	};

	#handleWheel = (e: WheelEvent) => {
		if (e.deltaY < 0) {
			this.#userScrolledAway = true;
		}
	};

	#handleTouchStart = (e: TouchEvent) => {
		this.#touchStartY = e.touches[0].clientY;
	};

	#handleTouchMove = (e: TouchEvent) => {
		if (this.#touchStartY === null) return;
		if (e.touches[0].clientY - this.#touchStartY > TOUCH_DEAD_ZONE_PX) {
			this.#userScrolledAway = true;
		}
	};

	#handleTouchEnd = () => {
		this.#touchStartY = null;
	};

	#handleKeyDown = (e: KeyboardEvent) => {
		if (e.key === 'PageUp' || e.key === 'ArrowUp' || e.key === 'Home') {
			this.#userScrolledAway = true;
		}
	};

	#handleScroll = () => {
		if (!this.#element || !this.#userScrolledAway) return;
		const { scrollTop, scrollHeight, clientHeight } = this.#element;
		if (scrollTop + clientHeight >= scrollHeight - RE_ENGAGE_THRESHOLD_PX) {
			this.#userScrolledAway = false;
		}
	};

	#pinToBottom = () => {
		this.#rafScheduled = false;
		if (!this.#userScrolledAway && this.#element) {
			this.#element.scrollTo({ top: this.#element.scrollHeight, behavior: 'auto' });
		}
	};

	#schedulePin = () => {
		if (this.#rafScheduled) return;
		this.#rafScheduled = true;
		requestAnimationFrame(this.#pinToBottom);
	};

	#setupObservers() {
		if (!this.#element) return;

		this.#element.addEventListener('wheel', this.#handleWheel, { passive: true });
		this.#element.addEventListener('touchstart', this.#handleTouchStart, { passive: true });
		this.#element.addEventListener('touchmove', this.#handleTouchMove, { passive: true });
		this.#element.addEventListener('touchend', this.#handleTouchEnd);
		this.#element.addEventListener('keydown', this.#handleKeyDown);
		this.#element.addEventListener('scroll', this.#handleScroll, { passive: true });

		this.#resizeObserver = new ResizeObserver(this.#schedulePin);
		this.#resizeObserver.observe(this.#element);

		this.#mutationObserver = new MutationObserver(this.#schedulePin);
		this.#mutationObserver.observe(this.#element, {
			childList: true,
			subtree: true,
			characterData: true,
		});
	}

	#cleanup() {
		this.#resizeObserver?.disconnect();
		this.#mutationObserver?.disconnect();

		if (this.#element) {
			this.#element.removeEventListener('wheel', this.#handleWheel);
			this.#element.removeEventListener('touchstart', this.#handleTouchStart);
			this.#element.removeEventListener('touchmove', this.#handleTouchMove);
			this.#element.removeEventListener('touchend', this.#handleTouchEnd);
			this.#element.removeEventListener('keydown', this.#handleKeyDown);
			this.#element.removeEventListener('scroll', this.#handleScroll);
		}

		this.#resizeObserver = null;
		this.#mutationObserver = null;
		this.#touchStartY = null;
		this.#rafScheduled = false;
	}
}

export function initStickToBottomContext(): StickToBottomContext {
	if (hasContext(STICK_TO_BOTTOM_CONTEXT_KEY)) {
		return getContext<StickToBottomContext>(STICK_TO_BOTTOM_CONTEXT_KEY);
	}
	const context = new StickToBottomContext();
	setContext(STICK_TO_BOTTOM_CONTEXT_KEY, context);
	return context;
}

export function getStickToBottomContext(): StickToBottomContext {
	const context = getContext<StickToBottomContext>(STICK_TO_BOTTOM_CONTEXT_KEY);
	if (!context) {
		throw new Error('StickToBottomContext must be used within a Conversation component');
	}
	return context;
}
