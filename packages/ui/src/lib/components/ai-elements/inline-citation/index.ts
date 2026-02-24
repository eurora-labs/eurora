import Root from './inline-citation.svelte';
import Text from './inline-citation-text.svelte';
import Card from './inline-citation-card.svelte';
import CardTrigger from './inline-citation-card-trigger.svelte';
import CardBody from './inline-citation-card-body.svelte';
import Quote from './inline-citation-quote.svelte';
import Source from './inline-citation-source.svelte';
import Carousel from './inline-citation-carousel.svelte';
import CarouselContent from './inline-citation-carousel-content.svelte';
import CarouselHeader from './inline-citation-carousel-header.svelte';
import CarouselIndex from './inline-citation-carousel-index.svelte';
import CarouselItem from './inline-citation-carousel-item.svelte';
import CarouselNext from './inline-citation-carousel-next.svelte';
import CarouselPrev from './inline-citation-carousel-prev.svelte';

export {
	Root,
	Text,
	Card,
	CardTrigger,
	CardBody,
	Quote,
	Source,
	Carousel,
	CarouselContent,
	CarouselHeader,
	CarouselIndex,
	CarouselItem,
	CarouselNext,
	CarouselPrev,
	//
	Root as InlineCitation,
	Text as InlineCitationText,
	Card as InlineCitationCard,
	CardTrigger as InlineCitationCardTrigger,
	CardBody as InlineCitationCardBody,
	Quote as InlineCitationQuote,
	Source as InlineCitationSource,
	Carousel as InlineCitationCarousel,
	CarouselContent as InlineCitationCarouselContent,
	CarouselHeader as InlineCitationCarouselHeader,
	CarouselIndex as InlineCitationCarouselIndex,
	CarouselItem as InlineCitationCarouselItem,
	CarouselNext as InlineCitationCarouselNext,
	CarouselPrev as InlineCitationCarouselPrev,
};

export {
	getCarouselContext,
	setCarouselContext,
	CarouselState,
} from './inline-citation-context.svelte.js';
