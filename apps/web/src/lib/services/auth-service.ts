import { createClient, type Client } from '@connectrpc/connect';
import { createGrpcWebTransport } from '@connectrpc/connect-web';
import {
	ProtoAuthService,
	type LoginRequest,
	type TokenResponse,
	type RefreshTokenRequest,
	type RegisterRequest,
	Provider,
	type ThirdPartyAuthUrlResponse,
	type LoginByLoginTokenRequest,
} from '@eurora/shared/proto/auth_service_pb.js';

const VITE_GRPC_API_URL: string = import.meta.env.VITE_GRPC_API_URL;

if (!VITE_GRPC_API_URL) {
	throw new Error('VITE_GRPC_API_URL environment variable is required but not defined');
}

export type EmailCheckResult =
	| { status: 'password' }
	| { status: 'oauth'; provider: 'google' | 'github' }
	| { status: 'not_found' };

class AuthService {
	private readonly client: Client<typeof ProtoAuthService>;
	constructor() {
		this.client = createClient(
			ProtoAuthService,
			createGrpcWebTransport({
				baseUrl: VITE_GRPC_API_URL,
				useBinaryFormat: true,
			}),
		);
	}

	public async login(data: LoginRequest): Promise<TokenResponse> {
		return await this.client.login(data);
	}

	public async register(data: RegisterRequest): Promise<TokenResponse> {
		return await this.client.register(data);
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

	public async checkEmail(email: string): Promise<EmailCheckResult> {
		const res = await this.client.checkEmail({ email });
		if (res.status === 'oauth' && res.provider !== undefined) {
			const providerName = res.provider === Provider.GOOGLE ? 'google' : 'github';
			return { status: 'oauth', provider: providerName };
		}
		if (res.status === 'password') {
			return { status: 'password' };
		}
		return { status: 'not_found' };
	}
}

export const authService = new AuthService();
export type {
	LoginRequest,
	TokenResponse,
	RegisterRequest,
	RefreshTokenRequest,
	Provider,
	ThirdPartyAuthUrlResponse,
	LoginByLoginTokenRequest,
};
