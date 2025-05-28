// Export auth service types and client
export {
	ProtoAuthServiceClientImpl,
	RegisterRequest,
	LoginRequest,
	RefreshTokenRequest,
	LoginResponse,
	EmailPasswordCredentials,
	ThirdPartyCredentials,
	Provider,
	type ProtoAuthService
} from './lib/gen/auth_service.js';
