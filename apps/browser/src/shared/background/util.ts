let cachedIsSafari: boolean | null = null;

export function isSafari() {
	if (cachedIsSafari !== null) {
		return cachedIsSafari;
	}
	const ua = navigator.userAgent || '';
	const vendor = navigator.vendor || '';
	const isAppleVendor = vendor.includes('Apple');
	const hasSafari = ua.includes('Safari');
	const isNotChromeLike =
		!ua.includes('Chrome') &&
		!ua.includes('Chromium') &&
		!ua.includes('CriOS') &&
		!ua.includes('Edg') &&
		!ua.includes('OPR') &&
		!ua.includes('FxiOS');

	cachedIsSafari = isAppleVendor && hasSafari && isNotChromeLike;
	return cachedIsSafari;
}
