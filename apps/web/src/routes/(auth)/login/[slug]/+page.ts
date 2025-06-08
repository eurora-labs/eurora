import type { PageLoad } from './$types';

export const prerender = false;
export const ssr = true;

export const load: PageLoad = ({ params }) => {
	return { slug: params.slug };
};
