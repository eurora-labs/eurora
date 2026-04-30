const HEX_PATTERN = /^#?([0-9a-f]{3}|[0-9a-f]{6})$/i;

function srgbToLinear(channel: number): number {
	const c = channel / 255;
	return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
}

export function parseHex(input: string): { r: number; g: number; b: number } | null {
	const match = HEX_PATTERN.exec(input.trim());
	if (!match) return null;
	let hex = match[1];
	if (hex.length === 3) {
		hex = hex[0] + hex[0] + hex[1] + hex[1] + hex[2] + hex[2];
	}
	const value = Number.parseInt(hex, 16);
	return {
		r: (value >> 16) & 0xff,
		g: (value >> 8) & 0xff,
		b: value & 0xff,
	};
}

export function relativeLuminance(rgb: { r: number; g: number; b: number }): number {
	return (
		0.2126 * srgbToLinear(rgb.r) +
		0.7152 * srgbToLinear(rgb.g) +
		0.0722 * srgbToLinear(rgb.b)
	);
}

// Returns null when `hex` cannot be parsed, signalling the caller should fall
// back to the existing theme rather than apply an override.
export function pickForeground(hex: string): 'white' | 'black' | null {
	const rgb = parseHex(hex);
	if (!rgb) return null;
	return relativeLuminance(rgb) > 0.5 ? 'black' : 'white';
}
