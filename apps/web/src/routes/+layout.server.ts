import type { LayoutServerLoad } from './$types';

export async function load({ locals }: Parameters<LayoutServerLoad>[0]) {
	return {
		user: locals.user,
	};
}
