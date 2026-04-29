import { create } from '@bufbuild/protobuf';
import { createClient, type Client } from '@connectrpc/connect';
import { createGrpcWebTransport } from '@connectrpc/connect-web';
import { InjectionToken } from '@eurora/shared/context';
import {
	AssociateLoginTokenRequestSchema,
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
	type VerifyEmailRequest,
} from '@eurora/shared/proto/auth_service_pb.js';
import * as Sentry from '@sentry/sveltekit';
import type { ConfigService } from '@eurora/shared/config/config-service';

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

	public async verifyEmail(data: VerifyEmailRequest): Promise<TokenResponse> {
		return await this.client.verifyEmail(data);
	}

	public async associateLoginToken(
		data: AssociateLoginTokenRequest,
		accessToken: string,
	): Promise<void> {
		await this.client.associateLoginToken(data, {
			headers: new Headers({ authorization: `Bearer ${accessToken}` }),
		});
	}

	public async associateDesktopLoginIfPending(
		accessToken: string,
		options: { consumeRedirect?: boolean } = {},
	): Promise<boolean> {
		const loginToken = sessionStorage.getItem('loginToken');
		if (!loginToken) return false;

		try {
			const request = create(AssociateLoginTokenRequestSchema, {
				codeChallenge: loginToken,
			});
			await this.associateLoginToken(request, accessToken);
			sessionStorage.removeItem('loginToken');
			sessionStorage.removeItem('challengeMethod');

			if (options.consumeRedirect) {
				const redirectUri = sessionStorage.getItem('deviceRedirectUri');
				if (redirectUri) {
					sessionStorage.removeItem('deviceRedirectUri');
					window.location.href = redirectUri;
				}
			}
			return true;
		} catch (err) {
			Sentry.captureException(err, { tags: { area: 'auth.associate-desktop' } });
			return false;
		}
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
	VerifyEmailRequest,
};
