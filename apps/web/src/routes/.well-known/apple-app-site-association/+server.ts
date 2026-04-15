import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

const association = {
	applinks: {
		details: [
			{
				appIDs: ['FKLH326P9A.com.eurora-labs.eurora.mobile.dev'],
				components: [
					{
						'/': '/mobile/callback*',
						comment: 'Mobile app auth callback deep link',
					},
				],
			},
		],
	},
	webcredentials: {
		apps: ['FKLH326P9A.com.eurora-labs.eurora.mobile.dev'],
	},
};

export const GET: RequestHandler = () => {
	return json(association, {
		headers: {
			'Content-Type': 'application/json',
		},
	});
};
