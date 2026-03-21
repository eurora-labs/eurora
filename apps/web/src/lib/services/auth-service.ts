import { createClient, type Client } from '@connectrpc/connect';
import { createGrpcWebTransport } from '@connectrpc/connect-web';
import { InjectionToken } from '@eurora/shared/context';
import {
	ProtoAuthService,
	type LoginRequest,
	type RegisterRequest,
	type TokenResponse,
	type RefreshTokenRequest,
	type CheckEmailRequest,
	type CheckEmailResponse,
	Provider,
	type ThirdPartyAuthUrlResponse,
	type LoginByLoginTokenRequest,
	type AssociateLoginTokenRequest,
} from '@eurora/shared/proto/auth_service_pb.js';
import type { ConfigService } from '$lib/services/config-service.js';

export class AuthService {
	private _client: Client<typeof ProtoAuthService> | null = null;
	private readonly config: ConfigService;

	constructor(config: ConfigService) {
		this.config = config;
	}

	private get client(): Client<typeof ProtoAuthService> {
		if (!this._client) {
			this._client = createClient(
				ProtoAuthService,
				createGrpcWebTransport({
					baseUrl: this.config.grpcApiUrl,
					useBinaryFormat: true,
				}),
			);
		}
		return this._client;
	}

	public async login(data: LoginRequest): Promise<TokenResponse> {
		return await this.client.login(data);
	}

	public async register(data: RegisterRequest): Promise<TokenResponse> {
		return await this.client.register(data);
	}

	public async checkEmail(data: CheckEmailRequest): Promise<CheckEmailResponse> {
		return await this.client.checkEmail(data);
	}

	public async refreshToken(data: RefreshTokenRequest): Promise<TokenResponse> {
		return await this.client.refreshToken(data);
	}

	public async getThirdPartyAuthUrl(provider: Provider): Promise<ThirdPartyAuthUrlResponse> {
		return await this.client.getThirdPartyAuthUrl({ provider });
	}

	public async loginByLoginToken(data: LoginByLoginTokenRequest): Promise<TokenResponse> {
		return await this.client.loginByLoginToken(data);
	}

	public async associateLoginToken(
		data: AssociateLoginTokenRequest,
		accessToken: string,
	): Promise<void> {
		await this.client.associateLoginToken(data, {
			headers: new Headers({ authorization: `Bearer ${accessToken}` }),
		});
	}
}

export const AUTH_SERVICE = new InjectionToken<AuthService>('AuthService');
export type {
	LoginRequest,
	RegisterRequest,
	TokenResponse,
	RefreshTokenRequest,
	CheckEmailRequest,
	CheckEmailResponse,
	Provider,
	ThirdPartyAuthUrlResponse,
	LoginByLoginTokenRequest,
	AssociateLoginTokenRequest,
};
