import { unwrap } from '$lib/bindings/result.js';
import { commands, events, type LoginOutcome } from '$lib/bindings/specta.bindings.js';
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
	 * Drives the entire OAuth flow on the Rust side: builds the auth URL with
	 * a fresh PKCE pair, opens the in-app browser via `tauri-plugin-appauth`,
	 * and exchanges the verifier for tokens once the redirect fires.
	 *
	 * The Rust side never throws for `USER_CANCELED` — it returns
	 * `{ kind: 'canceled' }` so the UI can silently return to idle without
	 * surfacing an error.
	 */
	async startLogin(): Promise<LoginOutcome> {
		const outcome = unwrap(await commands.authStartLogin());
		if (outcome.kind === 'success') {
			await this.fetchProfile();
		}
		return outcome;
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
