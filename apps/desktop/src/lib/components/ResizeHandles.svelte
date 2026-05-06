<script lang="ts">
	import { getCurrentWindow } from '@tauri-apps/api/window';

	type ResizeDirection =
		| 'North'
		| 'South'
		| 'East'
		| 'West'
		| 'NorthEast'
		| 'NorthWest'
		| 'SouthEast'
		| 'SouthWest';

	const appWindow = getCurrentWindow();

	function start(direction: ResizeDirection) {
		return (event: PointerEvent) => {
			if (event.button !== 0) return;
			event.preventDefault();
			appWindow.startResizeDragging(direction);
		};
	}
</script>

<!--
	Edges run the full width/height of the OS window rect at z-index 50;
	corners overlap them at z-index 51 so diagonal-resize wins the hit test
	in the 20x20 corner zone. Handles use viewport-fixed positioning so they
	cover the Linux shadow gutter and the full Windows window rect (DWM
	visually trims the rounded corners; clicks beyond fall through naturally).

	Rendered only on Linux/Windows; macOS handles edge resize natively.
-->

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="handle edge edge-n" onpointerdown={start('North')}></div>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="handle edge edge-s" onpointerdown={start('South')}></div>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="handle edge edge-e" onpointerdown={start('East')}></div>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="handle edge edge-w" onpointerdown={start('West')}></div>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="handle corner corner-nw" onpointerdown={start('NorthWest')}></div>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="handle corner corner-ne" onpointerdown={start('NorthEast')}></div>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="handle corner corner-sw" onpointerdown={start('SouthWest')}></div>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="handle corner corner-se" onpointerdown={start('SouthEast')}></div>

<style>
	.handle {
		position: fixed;
	}

	.edge {
		z-index: 50;
	}

	.corner {
		z-index: 51;
		width: 20px;
		height: 20px;
	}

	.edge-n {
		top: 0;
		right: 0;
		left: 0;
		height: 8px;
		cursor: ns-resize;
	}
	.edge-s {
		right: 0;
		bottom: 0;
		left: 0;
		height: 8px;
		cursor: ns-resize;
	}
	.edge-e {
		top: 0;
		right: 0;
		bottom: 0;
		width: 8px;
		cursor: ew-resize;
	}
	.edge-w {
		top: 0;
		bottom: 0;
		left: 0;
		width: 8px;
		cursor: ew-resize;
	}

	.corner-nw {
		top: 0;
		left: 0;
		cursor: nwse-resize;
	}
	.corner-ne {
		top: 0;
		right: 0;
		cursor: nesw-resize;
	}
	.corner-sw {
		bottom: 0;
		left: 0;
		cursor: nesw-resize;
	}
	.corner-se {
		right: 0;
		bottom: 0;
		cursor: nwse-resize;
	}
</style>
