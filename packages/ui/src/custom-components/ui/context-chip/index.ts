import { type VariantProps, tv } from 'tailwind-variants';

export { default as ContextChip } from './context-chip.svelte';
export const contextChipVariants = tv({
	base: 'inline-block w-fit items-center gap-2 mx-2 p-2 text-[40px] leading-[40px] rounded-2xl backdrop-blur-md text-black/70',
	variants: {
		variant: {
			default: 'bg-white/30',
			primary: 'bg-primary/30 text-primary-foreground',
			secondary: 'bg-secondary/30 text-secondary-foreground',
			destructive: 'bg-destructive/30 text-destructive-foreground',
			outline: 'border border-input bg-transparent'
		}
	},
	defaultVariants: {
		variant: 'default'
	}
});

export type Variant = VariantProps<typeof contextChipVariants>['variant'];
