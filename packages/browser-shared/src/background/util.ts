let cachedIsSafari: boolean | null = null;

export function isSafari() {
	if (cachedIsSafari !== null) {
		return cachedIsSafari;
	}
	// navigator is available in both background pages and service workers
	const ua = navigator.userAgent || '';
	const vendor = navigator.vendor || '';

	// Safari reports vendor as "Apple Computer, Inc."
	// It also includes "Safari" in UA but not "Chrome", "Chromium", "Edg", etc.
	const isAppleVendor = vendor.includes('Apple');
	const hasSafari = ua.includes('Safari');
	const isNotChromeLike =
		!ua.includes('Chrome') &&
		!ua.includes('Chromium') &&
		!ua.includes('CriOS') && // Chrome on iOS
		!ua.includes('Edg') && // Edge
		!ua.includes('OPR') && // Opera
		!ua.includes('FxiOS'); // Firefox on iOS

	cachedIsSafari = isAppleVendor && hasSafari && isNotChromeLike;
	return cachedIsSafari;
}
