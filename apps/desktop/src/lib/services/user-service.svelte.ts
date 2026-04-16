import { InjectionToken } from '@eurora/shared/context';
import type { LoginToken } from '$lib/bindings/bindings.js';
import type { TaurpcService } from '$lib/bindings/taurpcService.js';

export class UserService {
	authenticated = $state(false);
	emailVerified = $state(false);
	email = $state('');
	displayName = $state<string | null>(null);
	role = $state('');

	readonly planLabel = $derived(this.role === 'Tier1' ? 'Pro' : 'Free');

	private readonly taurpc: TaurpcService;
	private readonly unlisteners: Promise<() => void>[] = [];

	constructor(taurpc: TaurpcService) {
		this.taurpc = taurpc;
	}

	private async fetchProfile() {
		const [e, d, r, ev] = await Promise.all([
			this.taurpc.auth.get_email(),
			this.taurpc.auth.get_display_name(),
			this.taurpc.auth.get_role(),
			this.taurpc.auth.is_email_verified(),
		]);
		this.authenticated = true;
		this.email = e;
		this.displayName = d;
		this.role = r;
		this.emailVerified = ev;
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
					this.emailVerified = claims.email_verified ?? false;
				} else {
					this.authenticated = false;
					this.emailVerified = false;
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

	async resendVerificationEmail(): Promise<void> {
		await this.taurpc.auth.resend_verification_email();
	}

	async checkVerification(): Promise<boolean> {
		await this.taurpc.auth.refresh_session();
		return this.emailVerified;
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
