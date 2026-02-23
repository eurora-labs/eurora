import Root from './VoiceSelector.svelte';
import Trigger from './VoiceSelectorTrigger.svelte';
import Content from './VoiceSelectorContent.svelte';
import Dialog from './VoiceSelectorDialog.svelte';
import Input from './VoiceSelectorInput.svelte';
import List from './VoiceSelectorList.svelte';
import Empty from './VoiceSelectorEmpty.svelte';
import Group from './VoiceSelectorGroup.svelte';
import Item from './VoiceSelectorItem.svelte';
import Shortcut from './VoiceSelectorShortcut.svelte';
import Gender from './VoiceSelectorGender.svelte';
import Accent from './VoiceSelectorAccent.svelte';
import Preview from './VoiceSelectorPreview.svelte';
import Name from './VoiceSelectorName.svelte';
import Description from './VoiceSelectorDescription.svelte';
import Attributes from './VoiceSelectorAttributes.svelte';

export {
	Root,
	Trigger,
	Content,
	Dialog,
	Input,
	List,
	Empty,
	Group,
	Item,
	Shortcut,
	Gender,
	Accent,
	Preview,
	Name,
	Description,
	Attributes,
	//
	Root as VoiceSelector,
	Trigger as VoiceSelectorTrigger,
	Content as VoiceSelectorContent,
	Dialog as VoiceSelectorDialog,
	Input as VoiceSelectorInput,
	List as VoiceSelectorList,
	Empty as VoiceSelectorEmpty,
	Group as VoiceSelectorGroup,
	Item as VoiceSelectorItem,
	Shortcut as VoiceSelectorShortcut,
	Gender as VoiceSelectorGender,
	Accent as VoiceSelectorAccent,
	Preview as VoiceSelectorPreview,
	Name as VoiceSelectorName,
	Description as VoiceSelectorDescription,
	Attributes as VoiceSelectorAttributes,
};

export type { GenderValue, AccentValue } from './voice-selector-context.svelte.js';
