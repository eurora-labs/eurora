import { SvelteKitAuth } from '@auth/sveltekit';
import Apple from '@auth/sveltekit/providers/apple';
import GitHub from '@auth/sveltekit/providers/github';
import Google from '@auth/sveltekit/providers/google';
import Credentials from '@auth/sveltekit/providers/credentials';
// import { TRUST_HOST } from '$env/static/private';

export const { handle, signIn, signOut } = SvelteKitAuth({
	// trustHost: TRUST_HOST == 'true',
	trustHost: true,
	providers: [
		Apple,
		GitHub,
		Google,
		Credentials({
			credentials: {
				username: { label: 'Username' },
				password: { label: 'Password', type: 'password' },
			},
			async authorize(creds, request) {
				const response = await fetch(request);
				if (!response.ok) return null;
				return (await response.json()) ?? null;
			},
		}),
	],
});
