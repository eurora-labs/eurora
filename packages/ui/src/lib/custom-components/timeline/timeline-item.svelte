<script lang="ts" module>
	export interface TimelineItemProps {
		id?: string;
		/**
		 * Accent colour for the active-state border, given as any CSS
		 * colour string. Falls back to the sidebar border colour when unset.
		 */
		color?: string | null;
		/**
		 * The currently-selected item in the rail. Renders scaled up with a
		 * thin square border in the accent colour framing the icon;
		 * non-active items render as a bare, slightly smaller icon so the
		 * active one reads at a glance.
		 */
		active?: boolean;
		iconSrc?: string | null;
		name?: string;
		/**
		 * Fires when the user clicks the item. The parent listbox owns
		 * the selection state — the item itself is purely a visual
		 * affordance plus a click target.
		 */
		onSelect?: () => void;
	}
</script>

<script lang="ts">
	let {
		id,
		color = null,
		active = false,
		iconSrc = null,
		name = '',
		onSelect,
	}: TimelineItemProps = $props();

	let accent = $derived(color ?? 'var(--sidebar-border)');
</script>

<!--
  Items follow the W3C listbox pattern: keyboard navigation lives on
  the parent `[role="listbox"]` container (which owns Tab focus and
  arrow-key handling via `aria-activedescendant`), so an option only
  needs a mouse click handler to be selectable. The Svelte a11y lint
  expects every clickable element to have a paired keydown handler;
  that rule doesn't model listboxes, hence the suppression below.
-->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<li
	{id}
	title={name}
	role="option"
	aria-selected={active}
	aria-current={active ? 'true' : undefined}
	class="cursor-pointer transition-transform duration-150 ease-out"
	style="transform: scale({active ? 1.1 : 0.9});"
	onclick={onSelect}
>
	<span class="grid place-items-center">
		<span
			class="col-start-1 row-start-1 size-8 rounded-md"
			class:ring-1={active}
			class:ring-offset-2={active}
			class:ring-offset-sidebar={active}
			style="--tw-ring-color: {accent};"
			aria-hidden="true"
		></span>
		{#if iconSrc}
			<img src={iconSrc} alt={name} class="col-start-1 row-start-1 size-8" />
		{:else}
			<div
				class="text-sidebar-foreground col-start-1 row-start-1 flex size-8 items-center justify-center text-sm font-medium"
			>
				{name.charAt(0).toUpperCase()}
			</div>
		{/if}
	</span>
</li>
