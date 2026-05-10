import { unwrap } from '$lib/bindings/result.js';
import {
	commands,
	events,
	type LoginOutcome,
	type Provider,
} from '$lib/bindings/specta.bindings.js';
import { InjectionToken } from '@eurora/shared/context';

export class UserService {
	authenticated = $state(false);
	initialized = $state(false);
	email = $state('');
	displayName = $state<string | null>(null);
	role = $state('');

	private readonly unlisteners: Promise<() => void>[] = [];

	private async fetchProfile() {
		const claims = unwrap(await commands.authGetAccessTokenPayload());
		this.authenticated = true;
		this.email = claims.email;
		this.displayName = claims.display_name ?? null;
		this.role = claims.role;
	}

	async init() {
		try {
			this.unlisteners.push(
				events.authStateChanged.listen((event) => {
					const { claims } = event.payload;
					if (claims) {
						this.authenticated = true;
						this.email = claims.email;
						this.displayName = claims.display_name ?? null;
						this.role = claims.role;
					} else {
						this.authenticated = false;
						this.email = '';
						this.displayName = null;
						this.role = '';
					}
				}),
			);

			const isAuth = unwrap(await commands.authIsAuthenticated());
			if (isAuth) {
				await this.fetchProfile();
			}
		} finally {
			this.initialized = true;
		}
	}

	async login(login: string, password: string): Promise<void> {
		unwrap(await commands.authLogin(login, password));
		await this.fetchProfile();
	}

	async register(email: string, password: string): Promise<void> {
		unwrap(await commands.authRegister(email, password));
		await this.fetchProfile();
	}

	async logout(): Promise<void> {
		unwrap(await commands.authLogout());
	}

	/**
	 * Drives the in-app browser OAuth flow on the Rust side: gets a
	 * provider authorization URL from the backend (built around a fresh
	 * PKCE pair), opens it via `tauri-plugin-appauth`, and exchanges
	 * the verifier for tokens once the redirect fires.
	 *
	 * The Rust side never throws for `USER_CANCELED` — it returns
	 * `{ kind: 'canceled' }` so the UI can silently return to idle without
	 * surfacing an error.
	 */
	async startLogin(provider: Provider = 'google'): Promise<LoginOutcome> {
		const outcome = unwrap(await commands.authStartLogin(provider));
		if (outcome.kind === 'success') {
			await this.fetchProfile();
		}
		return outcome;
	}

	/**
	 * Sign in with Google. Tries the native iOS / Android Google SDK
	 * first (via `tauri-plugin-google-auth`) and falls back to the
	 * in-app browser flow when the native path isn't available
	 * (Android without Play Services, missing client-ID configuration,
	 * desktop dev builds). The user only ever sees the fallback if the
	 * native path can't run — successful native sign-in returns
	 * immediately without surfacing the browser.
	 */
	async signInWithGoogle(): Promise<LoginOutcome> {
		const native = unwrap(await commands.authStartLoginGoogleNative());
		if (native.kind === 'native_unavailable') {
			return await this.startLogin('google');
		}
		if (native.kind === 'success') {
			await this.fetchProfile();
		}
		return native;
	}

	async refreshSession(): Promise<void> {
		unwrap(await commands.authRefreshSession());
	}

	destroy() {
		for (const p of this.unlisteners) {
			p.then((unlisten) => unlisten());
		}
		this.unlisteners.length = 0;
	}
}

export const USER_SERVICE = new InjectionToken<UserService>('UserService');
