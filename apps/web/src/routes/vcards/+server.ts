import { redirect } from '@sveltejs/kit';

/**
 * @type {import('@sveltejs/kit').RequestHandler}
 */
export function GET() {
	// Redirect to the vCards listing page
	throw redirect(302, '/vcards');
}