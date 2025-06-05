import { createGrpcWebTransport } from '@connectrpc/connect-web';
import { createClient, type Client } from '@connectrpc/connect';
import {
	ProtoAuthService,
	type LoginRequest,
	type TokenResponse,
	type RefreshTokenRequest,
	type RegisterRequest,
} from '@eurora/proto/auth_service';

class AuthService {
	private readonly client: Client<typeof ProtoAuthService>;
	private readonly headers: Headers;
	constructor() {
		this.headers = new Headers();
		// this.headers.set('Access-Control-Allow-Origin', '*');
		// this.headers.set('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
		// this.headers.set('Access-Control-Allow-Headers', 'Content-Type');
		// this.headers.set('Access-Control-Allow-Credentials', 'true');
		this.client = createClient(
			ProtoAuthService,
			createGrpcWebTransport({
				baseUrl: 'https://api.eurora-labs.com',
				// baseUrl: 'http://localhost:50051',
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
}

export const authService = new AuthService();
export type { LoginRequest, TokenResponse, RegisterRequest, RefreshTokenRequest };
