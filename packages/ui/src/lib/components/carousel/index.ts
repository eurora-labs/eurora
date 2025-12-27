import Content from '$lib/components/carousel/carousel-content.svelte';
import Item from '$lib/components/carousel/carousel-item.svelte';
import Next from '$lib/components/carousel/carousel-next.svelte';
import Previous from '$lib/components/carousel/carousel-previous.svelte';
import Root from '$lib/components/carousel/carousel.svelte';

export {
	Root,
	Content,
	Item,
	Previous,
	Next,
	//
	Root as Carousel,
	Content as CarouselContent,
	Item as CarouselItem,
	Previous as CarouselPrevious,
	Next as CarouselNext,
};
