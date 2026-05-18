// Terminal styling for the dev scripts.
//
// Color decisions follow the de-facto NO_COLOR convention
// (https://no-color.org): respect $NO_COLOR if set to any non-empty
// value, and only emit escapes when stdout is a TTY. CI logs and
// piped output stay clean automatically.
//
// Node sets stdout to UTF-8 on every supported platform, so the check
// (✓), cross (✗) and arrow (↳) glyphs render correctly without the
// console code-page workaround the legacy PowerShell version needed.

const useColor = Boolean(process.stdout.isTTY) && !process.env.NO_COLOR;

const wrap = (open, close) => (s) => (useColor ? `\x1b[${open}m${s}\x1b[${close}m` : String(s));

export const color = {
	green: wrap('0;32', '0'),
	red: wrap('0;31', '0'),
	yellow: wrap('0;33', '0'),
	bold: wrap('1', '22'),
	dim: wrap('2', '22'),
};

export const glyph = {
	check: '✓',
	cross: '✗',
	arrow: '↳',
};
