import { InjectionToken } from '@eurora/shared/context';
import type { TaurpcService } from '$lib/bindings/taurpcService.js';

export class UserService {
	authenticated = $state(false);
	username = $state('');
	email = $state('');
	role = $state('');

	readonly planLabel = $derived(this.role === 'Tier1' ? 'Pro' : 'Free');

	private readonly taurpc: TaurpcService;

	constructor(taurpc: TaurpcService) {
		this.taurpc = taurpc;
	}

	async init() {
		this.authenticated = await this.taurpc.auth.is_authenticated();

		if (this.authenticated) {
			const [u, e, r] = await Promise.all([
				this.taurpc.auth.get_username(),
				this.taurpc.auth.get_email(),
				this.taurpc.auth.get_role(),
			]);
			this.username = u;
			this.email = e;
			this.role = r;
		}
	}
}

export const USER_SERVICE = new InjectionToken<UserService>('UserService');
