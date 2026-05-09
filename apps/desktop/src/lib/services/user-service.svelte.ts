import { InjectionToken } from '@eurora/shared/context';
import { commands, events, type LoginToken } from '$lib/bindings/specta.bindings.js';
import type { TelemetryService } from '$lib/services/telemetry-service.svelte.js';

// tauri-specta wraps every `Result<T, E>`-returning command in a tagged
// `{ status: "ok" | "error" }` envelope. The rest of this service treats
// command failures as exceptions (matching the old taurpc UX), so unwrap
// the envelope at the boundary and surface the backend message via `Error`.
type CommandResult<T, E> = { status: 'ok'; data: T } | { status: 'error'; error: E };

function unwrap<T>(result: CommandResult<T, string>): T {
	if (result.status === 'error') throw new Error(result.error);
	return result.data;
}

export class UserService {
	authenticated = $state(false);
	emailVerified = $state(false);
	email = $state('');
	displayName = $state<string | null>(null);
	role = $state('');

	readonly planLabel = $derived(this.role === 'Tier1' ? 'Pro' : 'Free');

	private readonly telemetry: TelemetryService;
	private readonly unlisteners: Promise<() => void>[] = [];

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

		this.unlisteners.push(
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

	destroy() {
		for (const p of this.unlisteners) {
			p.then((unlisten) => unlisten());
		}
		this.unlisteners.length = 0;
	}
}

export const USER_SERVICE = new InjectionToken<UserService>('UserService');
