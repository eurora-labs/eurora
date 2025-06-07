import { createGrpcWebTransport } from '@connectrpc/connect-web';
import { createClient, type Client } from '@connectrpc/connect';
import {
	ProtoAuthService,
	type LoginRequest,
	type TokenResponse,
	type RefreshTokenRequest,
	type RegisterRequest,
	Provider,
	type ThirdPartyAuthUrlResponse,
} from '@eurora/proto/auth_service';

class AuthService {
	private readonly client: Client<typeof ProtoAuthService>;
	constructor() {
		this.client = createClient(
			ProtoAuthService,
			createGrpcWebTransport({
				baseUrl: import.meta.env.VITE_API_BASE_URL || 'http://localhost:50051',
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
}

export const authService = new AuthService();
export type {
	LoginRequest,
	TokenResponse,
	RegisterRequest,
	RefreshTokenRequest,
	Provider,
	ThirdPartyAuthUrlResponse,
};
