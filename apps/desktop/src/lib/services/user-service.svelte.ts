import { ListenerBag } from '$lib/bindings/listeners.js';
import { unwrap } from '$lib/bindings/result.js';
import { commands, events, type LoginToken } from '$lib/bindings/specta.bindings.js';
import { InjectionToken } from '@eurora/shared/context';
import type { TelemetryService } from '$lib/services/telemetry-service.svelte.js';

export class UserService {
	authenticated = $state(false);
	emailVerified = $state(false);
	email = $state('');
	displayName = $state<string | null>(null);
	role = $state('');

	readonly planLabel = $derived(this.role === 'Tier1' ? 'Pro' : 'Free');

	private readonly telemetry: TelemetryService;
	private readonly listeners = new ListenerBag();

	constructor(telemetry: TelemetryService) {
		this.telemetry = telemetry;
	}

	private async fetchProfile() {
		const claims = unwrap(await commands.authGetAccessTokenPayload());
		this.authenticated = true;
		this.email = claims.email;
		this.displayName = claims.display_name ?? null;
		this.role = claims.role;
		this.emailVerified = claims.email_verified ?? false;
		this.telemetry.identify({
			email: this.email,
			displayName: this.displayName,
			role: this.role,
		});
	}

	async init() {
		const isAuth = unwrap(await commands.authIsAuthenticated());

		if (isAuth) {
			await this.fetchProfile();
		}

		this.listeners.add(
			events.authStateChanged.listen((event) => {
				const { claims } = event.payload;
				if (claims) {
					this.authenticated = true;
					this.email = claims.email;
					this.displayName = claims.display_name ?? null;
					this.role = claims.role;
					this.emailVerified = claims.email_verified ?? false;
					this.telemetry.identify({
						email: this.email,
						displayName: this.displayName,
						role: this.role,
					});
				} else {
					this.authenticated = false;
					this.emailVerified = false;
					this.email = '';
					this.displayName = null;
					this.role = '';
					this.telemetry.reset();
				}
			}),
		);
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

	async getLoginToken(): Promise<LoginToken> {
		return unwrap(await commands.authGetLoginToken());
	}

	async pollForLogin(): Promise<boolean> {
		const success = unwrap(await commands.authPollForLogin());
		if (success) {
			await this.fetchProfile();
		}
		return success;
	}

	async resendVerificationEmail(): Promise<void> {
		unwrap(await commands.authResendVerificationEmail());
	}

	async checkVerification(): Promise<boolean> {
		unwrap(await commands.authRefreshSession());
		const claims = unwrap(await commands.authGetAccessTokenPayload());
		this.emailVerified = claims.email_verified ?? false;
		return this.emailVerified;
	}

	async refreshSession(): Promise<void> {
		unwrap(await commands.authRefreshSession());
	}

	destroy(): Promise<void> {
		return this.listeners.destroy();
	}
}

export const USER_SERVICE = new InjectionToken<UserService>('UserService');
