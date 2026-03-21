import { InjectionToken } from '@eurora/shared/context';
import type { LoginToken } from '$lib/bindings/bindings.js';
import type { TaurpcService } from '$lib/bindings/taurpcService.js';

export class UserService {
	authenticated = $state(false);
	username = $state('');
	email = $state('');
	role = $state('');

	readonly planLabel = $derived(this.role === 'Tier1' ? 'Pro' : 'Free');

	private readonly taurpc: TaurpcService;
	private readonly unlisteners: Promise<() => void>[] = [];

	constructor(taurpc: TaurpcService) {
		this.taurpc = taurpc;
	}

	private async fetchProfile() {
		const [u, e, r] = await Promise.all([
			this.taurpc.auth.get_username(),
			this.taurpc.auth.get_email(),
			this.taurpc.auth.get_role(),
		]);
		this.authenticated = true;
		this.username = u;
		this.email = e;
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
					this.username = claims.email;
					this.email = claims.email;
					this.role = claims.role;
				} else {
					this.authenticated = false;
					this.username = '';
					this.email = '';
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

	async getLoginToken(): Promise<LoginToken> {
		return this.taurpc.auth.get_login_token();
	}

	async pollForLogin(): Promise<boolean> {
		const success = await this.taurpc.auth.poll_for_login();
		if (success) {
			await this.fetchProfile();
		}
		return success;
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
