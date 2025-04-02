import { error } from '@sveltejs/kit';
import fs from 'fs';
import path from 'path';
export const prerender = true;
/**
 * @type {import('@sveltejs/kit').RequestHandler}
 */
export async function GET() {
    const name = "maitte_oliveira";

    // In production, it's in the dist directory (as configured in svelte.config.js)
    const staticDir = path.join(process.cwd(), 'static');
    const vcardPath = path.join(staticDir, 'vcards', `${name}.vcf`);

    // Check if the file exists
    if (!fs.existsSync(vcardPath)) {
        throw error(404, `vCard not found for ${name}`);
    }

    // Read the vCard file
    const vcardContent = fs.readFileSync(vcardPath, 'utf-8');

    // Return the vCard content with appropriate headers
    return new Response(vcardContent, {
			headers: {
				'Content-Type': 'text/vcard',
				'Content-Disposition': `attachment; filename="${name}.vcf"`
			}
		});
}
