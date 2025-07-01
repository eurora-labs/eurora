# Storybook Guidelines for Eurora UI Components

This document outlines the standards and best practices for creating Storybook stories in the Eurora UI package, based on analysis of existing examples and industry best practices.

## File Structure and Organization

### Directory Structure

- **Component-based organization**: Each component should have its own directory under `packages/ui/src/stories/`
- **Naming convention**: Directory names should match the component name in kebab-case (e.g., `button/`, `context-chip/`, `video-card/`)
- **Story file naming**: Use `ComponentName.stories.svelte` format (e.g., `Button.stories.svelte`, `VideoCard.stories.svelte`)

### File Organization Pattern

```
packages/ui/src/stories/
├── component-name/
│   ├── ComponentName.stories.svelte          # Interactive story with controls
│   ├── AllComponentName.stories.svelte       # Showcase of all variants (optional)
│   └── [additional story files if needed]
```

### Dual Story Pattern (Recommended for Complex Components)

For components with many variants, states, or configurations, consider implementing a dual story pattern:

1. **Interactive Story** (`ComponentName.stories.svelte`):
    - Single story with full Storybook controls
    - Allows developers to test all component properties
    - Primary entry point for component testing

2. **Showcase Story** (`AllComponentName.stories.svelte`):
    - Comprehensive display of all variants and states
    - No interactive controls (set `controls: { disable: true }`)
    - Organized with clear section headings
    - Serves as visual documentation and design reference

**When to use dual pattern:**

- Components with 4+ variants or sizes
- Components with multiple distinct states (loading, disabled, error)
- Components that benefit from side-by-side comparison
- Complex components where seeing all options together aids understanding

## Story File Structure

### Required Sections

#### 1. Module Script Block

```svelte
<script module lang="ts">
	import { ComponentName } from '$lib/components/path/to/component';
	import { defineMeta, type StoryContext, type Args } from '@storybook/addon-svelte-csf';

	const { Story } = defineMeta({
		title: 'Category / ComponentName',
		component: ComponentName,
		parameters: {
			docs: {
				description: {
					component:
						'Clear, concise description of the component purpose and functionality.',
				},
			},
			layout: 'centered', // or 'padded' for larger components
		},
		argTypes: {
			// Define controls for component props
		},
		args: {
			// Default values for props
		},
	});
</script>
```

#### 2. Component Script Block

```svelte
<script lang="ts">
	// Import icons, utilities, and other dependencies
	import { Icon1, Icon2 } from '@lucide/svelte';
</script>
```

#### 3. Story Definitions

```svelte
<!-- Story Name -->
<Story name="StoryName">
	<!-- Story content -->
</Story>
```

#### 4. Interactive Template (Optional)

```svelte
{#snippet template({ ...args }: Args<typeof Story>, _context: StoryContext<typeof Story>)}
	<!-- Interactive story with controls -->
{/snippet}
```

## Story Categories and Naming

### Category Hierarchy

- **Components**: General UI components (`Components / Button`, `Components / VideoCard`)
- **Inputs**: Form and input-related components (`Inputs / ContextChip`)
- **Layout**: Layout and structural components
- **Navigation**: Navigation-related components
- **Feedback**: Alerts, notifications, loading states

### Story Naming Conventions

- **Default**: Basic component usage
- **Variants**: Different visual styles
- **Sizes**: Different size options
- **States**: Different component states (disabled, loading, error)
- **With [Feature]**: Component with specific features (`With Icons`, `With Groups`)
- **[Specific Use Case]**: Descriptive names for specific scenarios

## Required Stories

### Single Story Pattern (Simple Components)

For components with few variants or simple configurations:

1. **Interactive**: Single story with controls for all component properties
2. **Additional Stories**: Specific use cases or complex examples as needed

### Dual Story Pattern (Complex Components)

For components with many variants, states, or configurations:

#### Interactive Story (`ComponentName.stories.svelte`)

- **Single Interactive Story**: Template with full Storybook controls
- **Purpose**: Allow developers to test all component properties dynamically
- **Title**: `Components / ComponentName` (main entry point)

#### Showcase Story (`AllComponentName.stories.svelte`)

- **Comprehensive Display**: All variants, sizes, states organized in sections
- **No Controls**: Set `controls: { disable: true }` in parameters
- **Clear Organization**: Use section headings and proper spacing
- **Title**: `Components / ComponentName / All [ComponentName]s`
- **Sections to Include**:
    1. **Default**: Basic component usage
    2. **Variants**: All available visual variants
    3. **Sizes**: All available size options (if applicable)
    4. **States**: Disabled, loading, error, active states
    5. **With Features**: Icons, links, special functionality
    6. **Use Cases**: Specific scenarios or configurations

### Component-Specific Considerations

- **Complex Components**: Empty states, loading states, error states
- **Layout Components**: Responsive behavior, alignment options
- **Form Components**: Validation states, different input types
- **Navigation Components**: Active states, nested structures

## Documentation Standards

### Component Descriptions

- **Purpose**: What the component does
- **Use cases**: When to use it
- **Key features**: Important functionality
- **Responsive behavior**: How it adapts to different screen sizes

### Story Descriptions

- Use clear, descriptive names
- Include context about when to use each variant
- Document any special behavior or interactions

### ArgTypes Configuration

```typescript
argTypes: {
	propName: {
		control: { type: 'select' | 'boolean' | 'text' | 'number' },
		options: ['option1', 'option2'], // for select controls
		description: 'Clear description of what this prop does'
	}
}
```

## Visual and UX Guidelines

### Layout and Spacing

- Use consistent container widths for similar components
- Apply appropriate spacing between story elements
- Use flexbox layouts for multiple variants: `<div class="flex flex-wrap gap-4">`

### Icon Usage

- Import icons from `@lucide/svelte`
- Use consistent icon sizing: `h-4 w-4` for most cases
- Apply appropriate spacing: `mr-2` for left icons, `ml-2` for right icons
- Use semantic color classes for colored icons: `text-blue-500`, `text-green-500`

### Content Guidelines

- Use realistic, meaningful content in examples
- Provide variety in content length to test different scenarios
- Use placeholder content that reflects real-world usage

## Accessibility Standards

### Required Practices

- Include proper ARIA attributes in examples
- Demonstrate keyboard navigation where applicable
- Show focus states and interactions
- Use semantic HTML elements
- Provide alternative text for images and icons

### Testing Scenarios

- Include stories that test accessibility features
- Demonstrate proper labeling and descriptions
- Show error states with appropriate messaging

## Technical Requirements

### Import Patterns

```svelte
// Component imports - use absolute paths from $lib
import { Component } from '$lib/components/path/to/component';

// Icon imports
import { IconName } from '@lucide/svelte';

// Storybook imports
import { defineMeta, type StoryContext, type Args } from '@storybook/addon-svelte-csf';
```

### TypeScript Usage

- Always use TypeScript for type safety
- Define proper types for story arguments
- Use type annotations for complex props

### Responsive Design

- Test components at different viewport sizes
- Include stories that demonstrate responsive behavior
- Use appropriate Tailwind CSS classes for responsive design

## Performance Considerations

### Optimization Guidelines

- Avoid heavy computations in story definitions
- Use efficient rendering patterns
- Minimize unnecessary re-renders
- Optimize image and video assets

### Asset Management

- Use appropriate video formats and sizes
- Optimize images for web display
- Consider loading states for heavy content

## Quality Assurance

### Review Checklist

- [ ] All required stories are present
- [ ] Component description is clear and accurate
- [ ] ArgTypes are properly configured
- [ ] Visual consistency across stories
- [ ] Accessibility features are demonstrated
- [ ] Responsive behavior is tested
- [ ] TypeScript types are correct
- [ ] Import paths are consistent
- [ ] Content is realistic and meaningful

### Testing Requirements

- Verify all stories render without errors
- Test interactive controls functionality
- Validate responsive behavior
- Check accessibility with screen readers
- Ensure consistent visual appearance

## Best Practices Summary

1. **Consistency**: Follow established patterns from existing stories
2. **Completeness**: Cover all component variants and states
3. **Clarity**: Use descriptive names and documentation
4. **Accessibility**: Include proper ARIA attributes and semantic HTML
5. **Responsiveness**: Test and demonstrate responsive behavior
6. **Performance**: Optimize assets and avoid unnecessary complexity
7. **Maintainability**: Use TypeScript and follow coding standards
8. **User-Focused**: Create stories that help developers understand component usage

## Examples and References

Refer to existing stories for implementation examples:

### Dual Story Pattern Examples

- [`Button.stories.svelte`](./button/Button.stories.svelte) - Interactive story with full controls
- [`AllButton.stories.svelte`](./button/AllButton.stories.svelte) - Comprehensive showcase of all button variants and states

### Single Story Pattern Examples

- [`Command.stories.svelte`](./launcher/Command.stories.svelte) - Complex component with multiple sub-components
- [`VideoCard.stories.svelte`](./video-card/VideoCard.stories.svelte) - Responsive component with media content

### Implementation Templates

#### Interactive Story Template

```svelte
<script module lang="ts">
	import { Component } from '$lib/components/path/to/component';
	import { defineMeta, type StoryContext, type Args } from '@storybook/addon-svelte-csf';

	const { Story } = defineMeta({
		title: 'Components / ComponentName',
		component: Component,
		argTypes: {
			// Define all component props with controls
		},
		args: {
			// Default values
		}
	});
</script>

{#snippet template({ ...args }: Args<typeof Story>, _context: StoryContext<typeof Story>)}
	<Component {...args}>
		{/* Dynamic content based on args */}
	</Component>
{/snippet}

<Story name="Interactive" children={template} />
```

#### Showcase Story Template

```svelte
<script module lang="ts">
	import { Component } from '$lib/components/path/to/component';
	import { defineMeta } from '@storybook/addon-svelte-csf';

	const { Story } = defineMeta({
		title: 'Components / ComponentName / All ComponentNames',
		component: Component,
		parameters: {
			controls: { disable: true },
		},
	});
</script>

<Story name="All ComponentName Examples">
	<div class="space-y-8 p-6">
		<div>
			<h2 class="text-lg font-semibold mb-4">Section Title</h2>
			<div class="flex flex-wrap gap-4">
				<!-- Component examples -->
			</div>
		</div>
		<!-- More sections -->
	</div>
</Story>
```

These guidelines ensure consistent, high-quality Storybook documentation that serves both developers and designers effectively.
