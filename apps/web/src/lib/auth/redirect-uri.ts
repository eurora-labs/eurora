const APP_SCHEME = 'eurora:';
const MOBILE_CALLBACK_PATH = '/mobile/callback';
const STORAGE_KEY = 'deviceRedirectUri';

/**
 * Validate a `redirect_uri` provided by an app client.
 *
 * Allowed shapes:
 *   - `eurora://...`              — custom URL scheme intercepted by the
 *                                    Tauri desktop runtime or by
 *                                    ASWebAuthenticationSession on iOS.
 *   - `<webOrigin>/mobile/callback` — universal-link fallback for mobile.
 *
 * Anything else (including subtle origin spoofing via userinfo, mismatched
 * paths, or unparseable strings) is rejected. This is the open-redirector
 * boundary — every code path that consumes a stored redirect URI must
 * re-validate, since `sessionStorage` is mutable from any same-origin script.
 */
export function validateAppRedirectUri(
	rawUri: string | null | undefined,
	webOrigin: string,
): string | null {
	if (!rawUri) return null;

	let parsed: URL;
	try {
		parsed = new URL(rawUri);
	} catch {
		return null;
	}

	if (parsed.protocol === APP_SCHEME) {
		return parsed.toString();
	}

	if (parsed.origin === webOrigin && parsed.pathname === MOBILE_CALLBACK_PATH) {
		return parsed.toString();
	}

	return null;
}

export function storeAppRedirectUri(rawUri: string | null | undefined): void {
	const validated = validateAppRedirectUri(rawUri, window.location.origin);
	if (validated) {
		sessionStorage.setItem(STORAGE_KEY, validated);
	}
}

export function peekAppRedirectUri(): string | null {
	return validateAppRedirectUri(sessionStorage.getItem(STORAGE_KEY), window.location.origin);
}

export function consumeAppRedirectUri(): string | null {
	const uri = peekAppRedirectUri();
	sessionStorage.removeItem(STORAGE_KEY);
	return uri;
}
