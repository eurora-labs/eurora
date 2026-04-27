import { InjectionToken } from '@eurora/shared/context';
import type { LoginOutcome } from '$lib/bindings/bindings.js';
import type { TaurpcService } from '$lib/bindings/taurpcService.js';

export class UserService {
	authenticated = $state(false);
	email = $state('');
	displayName = $state<string | null>(null);
	role = $state('');

	private readonly taurpc: TaurpcService;
	private readonly unlisteners: Promise<() => void>[] = [];

	constructor(taurpc: TaurpcService) {
		this.taurpc = taurpc;
	}

	private async fetchProfile() {
		const [e, d, r] = await Promise.all([
			this.taurpc.auth.get_email(),
			this.taurpc.auth.get_display_name(),
			this.taurpc.auth.get_role(),
		]);
		this.authenticated = true;
		this.email = e;
		this.displayName = d;
		this.role = r;
	}

	async init() {
		const isAuth = await this.taurpc.auth.is_authenticated();

		if (isAuth) {
			await this.fetchProfile();
		}

		this.unlisteners.push(
			this.taurpc.auth.auth_state_changed.on((claims) => {
				if (claims) {
					this.authenticated = true;
					this.email = claims.email;
					this.displayName = claims.display_name;
					this.role = claims.role;
				} else {
					this.authenticated = false;
					this.email = '';
					this.displayName = null;
					this.role = '';
				}
			}),
		);
	}

	async login(login: string, password: string): Promise<void> {
		await this.taurpc.auth.login(login, password);
		await this.fetchProfile();
	}

	async register(email: string, password: string): Promise<void> {
		await this.taurpc.auth.register(email, password);
		await this.fetchProfile();
	}

	async logout(): Promise<void> {
		await this.taurpc.auth.logout();
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
		const outcome = await this.taurpc.auth.start_login();
		if (outcome.kind === 'success') {
			await this.fetchProfile();
		}
		return outcome;
	}

	async refreshSession(): Promise<void> {
		await this.taurpc.auth.refresh_session();
	}

	destroy() {
		for (const p of this.unlisteners) {
			p.then((unlisten) => unlisten());
		}
		this.unlisteners.length = 0;
	}
}

export const USER_SERVICE = new InjectionToken<UserService>('UserService');
