import { error } from '@sveltejs/kit';
import fs from 'fs';
import path from 'path';

/**
 * @type {import('@sveltejs/kit').RequestHandler}
 */
export async function GET({ params }) {
	const { name } = params as { name: string };

	// Sanitize the name parameter to prevent directory traversal attacks
	const sanitizedName = name.replace(/[^a-zA-Z0-9_-]/g, '');

	if (sanitizedName !== name) {
		throw error(400, 'Invalid vCard name');
	}

	// Get the path to the static directory
	// In development, it's in the static directory
	// In production, it's in the dist directory (as configured in svelte.config.js)
	const staticDir = path.join(process.cwd(), 'static');
	const vcardPath = path.join(staticDir, 'vcards', `${sanitizedName}.vcf`);

	// Check if the file exists
	if (!fs.existsSync(vcardPath)) {
		throw error(404, `vCard not found for ${sanitizedName}`);
	}

	// Read the vCard file
	const vcardContent = fs.readFileSync(vcardPath, 'utf-8');

	// Return the vCard content with appropriate headers
	return new Response(vcardContent, {
		headers: {
			'Content-Type': 'text/vcard',
			'Content-Disposition': `attachment; filename="${sanitizedName}.vcf"`
		}
	});
}
