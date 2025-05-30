import { createGrpcWebTransport } from '@connectrpc/connect-web';
import { createClient, type Client } from '@connectrpc/connect';
import {
	ProtoAuthService,
	type LoginRequest,
	type LoginResponse,
	type RefreshTokenRequest,
	type RegisterRequest
} from '@eurora/proto/auth_service';

class AuthService {
	private readonly client: Client<typeof ProtoAuthService>;
	constructor() {
		this.client = createClient(
			ProtoAuthService,
			createGrpcWebTransport({
				baseUrl: 'http://localhost:50051',
				useBinaryFormat: true
			})
		);
	}

	public async login(data: LoginRequest): Promise<LoginResponse> {
		return await this.client.login(data);
	}

	public async register(data: RegisterRequest): Promise<LoginResponse> {
		return await this.client.register(data);
	}

	public async refreshToken(data: RefreshTokenRequest): Promise<LoginResponse> {
		return await this.client.refreshToken(data);
	}
}

export const authService = new AuthService();
export type { LoginRequest, LoginResponse, RegisterRequest, RefreshTokenRequest };
