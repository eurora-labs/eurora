import Root from './terminal.svelte';
import Header from './terminal-header.svelte';
import Title from './terminal-title.svelte';
import Status from './terminal-status.svelte';
import Actions from './terminal-actions.svelte';
import Content from './terminal-content.svelte';
import CopyButton from './terminal-copy-button.svelte';
import ClearButton from './terminal-clear-button.svelte';

export {
	Root,
	Header,
	Title,
	Status,
	Actions,
	Content,
	CopyButton,
	ClearButton,
	//
	Root as Terminal,
	Header as TerminalHeader,
	Title as TerminalTitle,
	Status as TerminalStatus,
	Actions as TerminalActions,
	Content as TerminalContent,
	CopyButton as TerminalCopyButton,
	ClearButton as TerminalClearButton,
};

export {
	TerminalState,
	getTerminalContext,
	setTerminalContext,
} from './terminal-context.svelte.js';
