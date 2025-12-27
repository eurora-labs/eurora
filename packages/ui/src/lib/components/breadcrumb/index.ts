import Ellipsis from '$lib/components/breadcrumb/breadcrumb-ellipsis.svelte';
import Item from '$lib/components/breadcrumb/breadcrumb-item.svelte';
import Link from '$lib/components/breadcrumb/breadcrumb-link.svelte';
import List from '$lib/components/breadcrumb/breadcrumb-list.svelte';
import Page from '$lib/components/breadcrumb/breadcrumb-page.svelte';
import Separator from '$lib/components/breadcrumb/breadcrumb-separator.svelte';
import Root from '$lib/components/breadcrumb/breadcrumb.svelte';

export {
	Root,
	Ellipsis,
	Item,
	Separator,
	Link,
	List,
	Page,
	//
	Root as Breadcrumb,
	Ellipsis as BreadcrumbEllipsis,
	Item as BreadcrumbItem,
	Separator as BreadcrumbSeparator,
	Link as BreadcrumbLink,
	List as BreadcrumbList,
	Page as BreadcrumbPage,
};
