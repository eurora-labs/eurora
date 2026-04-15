import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

const assetlinks = [
	{
		relation: ['delegate_permission/common.handle_all_urls'],
		target: {
			namespace: 'android_app',
			package_name: 'com.eurora_labs.eurora.mobile.dev',
			sha256_cert_fingerprints: [
				'TODO:REPLACE_WITH_YOUR_SIGNING_CERTIFICATE_SHA256_FINGERPRINT',
			],
		},
	},
];

export const GET: RequestHandler = () => {
	return json(assetlinks, {
		headers: {
			'Content-Type': 'application/json',
		},
	});
};
