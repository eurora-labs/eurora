import Content from '$lib/components/pagination/pagination-content.svelte';
import Ellipsis from '$lib/components/pagination/pagination-ellipsis.svelte';
import Item from '$lib/components/pagination/pagination-item.svelte';
import Link from '$lib/components/pagination/pagination-link.svelte';
import NextButton from '$lib/components/pagination/pagination-next-button.svelte';
import PrevButton from '$lib/components/pagination/pagination-prev-button.svelte';
import Root from '$lib/components/pagination/pagination.svelte';

export {
	Root,
	Content,
	Item,
	Link,
	PrevButton,
	NextButton,
	Ellipsis,
	//
	Root as Pagination,
	Content as PaginationContent,
	Item as PaginationItem,
	Link as PaginationLink,
	PrevButton as PaginationPrevButton,
	NextButton as PaginationNextButton,
	Ellipsis as PaginationEllipsis,
};
