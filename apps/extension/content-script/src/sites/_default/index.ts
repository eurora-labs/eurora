export function mainDefault() {
	// Minimal, zero-conf features that are safe on any site.
	// Keep <3–5 KB, no polling, no network.
	// Example: keyboard helpers, simple DOM overlay toggled via action icon.
	console.log('Default site features loaded');
	alert('Hello, world from main default!');
	// const badge = document.createElement('div');
	// badge.className = 'ext-default-pill';
	// badge.textContent = 'Toolkit';
	// document.documentElement.appendChild(badge);
}
