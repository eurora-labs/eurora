export const prerender = true;
/**
 * @type {import('@sveltejs/kit').RequestHandler}
 */
export async function GET() {
	// Redirect to the .vcf endpoint
	return new Response(null, {
		status: 301,
		headers: {
			Location: 'https://eurora-labs.com'
		}
	});
}
