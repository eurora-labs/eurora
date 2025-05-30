import {
	ProtoAuthServiceClientImpl,
	RegisterRequest,
	LoginRequest,
	RefreshTokenRequest,
	LoginResponse,
	EmailPasswordCredentials,
	type ProtoAuthService
} from '@eurora/proto/auth_service';

// Simple gRPC-Web transport implementation using fetch
class GrpcWebRpc {
	private readonly host: string;

	constructor(host: string) {
		this.host = host;
	}

	async request(service: string, method: string, data: Uint8Array): Promise<Uint8Array> {
		const url = `${this.host}/${service}/${method}`;

		try {
			const response = await fetch(url, {
				method: 'POST',
				headers: {
					'Content-Type': 'application/grpc-web+proto',
					Accept: 'application/grpc-web+proto',
					'X-Grpc-Web': '1'
				},
				body: data
			});

			if (!response.ok) {
				const errorText = await response.text();
				throw new Error(
					`gRPC request failed: ${response.status} ${response.statusText} - ${errorText}`
				);
			}

			const responseData = await response.arrayBuffer();
			return new Uint8Array(responseData);
		} catch (error) {
			console.error('gRPC request error:', error);
			throw error;
		}
	}
}

export interface RegisterData {
	username: string;
	email: string;
	password: string;
	displayName?: string;
}

export interface LoginData {
	login: string; // username or email
	password: string;
}

export interface AuthTokens {
	accessToken: string;
	refreshToken: string;
	expiresIn: number;
}

export class AuthService {
	private client: ProtoAuthService;
	private readonly baseUrl: string;

	constructor(baseUrl: string = 'http://localhost:8080') {
		this.baseUrl = baseUrl;
		const rpc = new GrpcWebRpc(baseUrl);
		this.client = new ProtoAuthServiceClientImpl(rpc);
	}

	/**
	 * Register a new user account
	 */
	async register(data: RegisterData): Promise<AuthTokens> {
		try {
			const request: RegisterRequest = {
				username: data.username,
				email: data.email,
				password: data.password,
				displayName: data.displayName
			};

			console.log('Sending registration request:', {
				username: request.username,
				email: request.email,
				displayName: request.displayName
			});

			const response = await this.client.Register(request);

			console.log('Registration successful');

			return {
				accessToken: response.accessToken,
				refreshToken: response.refreshToken,
				expiresIn: response.expiresIn
			};
		} catch (error) {
			console.error('Registration failed:', error);
			throw new Error(this.extractErrorMessage(error));
		}
	}

	/**
	 * Login with email/username and password
	 */
	async login(data: LoginData): Promise<AuthTokens> {
		try {
			const credentials: EmailPasswordCredentials = {
				login: data.login,
				password: data.password
			};

			const request: LoginRequest = {
				emailPassword: credentials,
				thirdParty: undefined
			};

			console.log('Sending login request for:', data.login);

			const response = await this.client.Login(request);

			console.log('Login successful');

			return {
				accessToken: response.accessToken,
				refreshToken: response.refreshToken,
				expiresIn: response.expiresIn
			};
		} catch (error) {
			console.error('Login failed:', error);
			throw new Error(this.extractErrorMessage(error));
		}
	}

	/**
	 * Refresh access token using refresh token
	 */
	async refreshToken(refreshToken: string): Promise<AuthTokens> {
		try {
			const request: RefreshTokenRequest = {
				refreshToken
			};

			console.log('Refreshing token');

			const response = await this.client.RefreshToken(request);

			console.log('Token refresh successful');

			return {
				accessToken: response.accessToken,
				refreshToken: response.refreshToken,
				expiresIn: response.expiresIn
			};
		} catch (error) {
			console.error('Token refresh failed:', error);
			throw new Error(this.extractErrorMessage(error));
		}
	}

	/**
	 * Extract error message from gRPC error
	 */
	private extractErrorMessage(error: any): string {
		if (error?.message) {
			// Try to extract meaningful error from gRPC error
			const message = error.message;
			if (message.includes('Registration failed:')) {
				return message.replace('gRPC request failed:', '').trim();
			}
			if (message.includes('Invalid credentials')) {
				return 'Invalid username/email or password';
			}
			if (message.includes('Username already exists')) {
				return 'Username is already taken';
			}
			if (message.includes('Email already exists')) {
				return 'Email is already registered';
			}
			return message;
		}
		if (typeof error === 'string') {
			return error;
		}
		return 'An unexpected error occurred. Please try again.';
	}
}

// Create a singleton instance
export const authService = new AuthService();

// Token storage utilities
export class TokenStorage {
	private static readonly ACCESS_TOKEN_KEY = 'eurora_access_token';
	private static readonly REFRESH_TOKEN_KEY = 'eurora_refresh_token';
	private static readonly EXPIRES_AT_KEY = 'eurora_expires_at';

	static saveTokens(tokens: AuthTokens): void {
		const expiresAt = Date.now() + tokens.expiresIn * 1000;

		localStorage.setItem(this.ACCESS_TOKEN_KEY, tokens.accessToken);
		localStorage.setItem(this.REFRESH_TOKEN_KEY, tokens.refreshToken);
		localStorage.setItem(this.EXPIRES_AT_KEY, expiresAt.toString());
	}

	static getAccessToken(): string | null {
		return localStorage.getItem(this.ACCESS_TOKEN_KEY);
	}

	static getRefreshToken(): string | null {
		return localStorage.getItem(this.REFRESH_TOKEN_KEY);
	}

	static isTokenExpired(): boolean {
		const expiresAt = localStorage.getItem(this.EXPIRES_AT_KEY);
		if (!expiresAt) return true;

		return Date.now() >= parseInt(expiresAt);
	}

	static clearTokens(): void {
		localStorage.removeItem(this.ACCESS_TOKEN_KEY);
		localStorage.removeItem(this.REFRESH_TOKEN_KEY);
		localStorage.removeItem(this.EXPIRES_AT_KEY);
	}

	static async refreshTokenIfNeeded(): Promise<boolean> {
		const refreshToken = this.getRefreshToken();
		if (!refreshToken || !this.isTokenExpired()) {
			return false;
		}

		try {
			const tokens = await authService.refreshToken(refreshToken);
			this.saveTokens(tokens);
			return true;
		} catch (error) {
			console.error('Failed to refresh token:', error);
			this.clearTokens();
			return false;
		}
	}
}
