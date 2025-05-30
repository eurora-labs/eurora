import { create } from '@bufbuild/protobuf';
import { createConnectTransport, createGrpcWebTransport } from '@connectrpc/connect-web';
import { createClient, type Client } from '@connectrpc/connect';
import {
	ProtoAuthService,
	type LoginRequest,
	type LoginResponse,
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
		try {
			const response = await this.client.login(data);
			return response;
		} catch (error) {
			throw error;
		}
	}

	public async register(data: RegisterRequest): Promise<LoginResponse> {
		try {
			const response = await this.client.register(data);
			return response;
		} catch (error) {
			throw error;
		}
	}
}

export const authService = new AuthService();
export type { LoginRequest, LoginResponse };
