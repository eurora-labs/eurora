import Root from './stack-trace.svelte';
import Header from './stack-trace-header.svelte';
import Error from './stack-trace-error.svelte';
import ErrorType from './stack-trace-error-type.svelte';
import ErrorMessage from './stack-trace-error-message.svelte';
import Frames from './stack-trace-frames.svelte';
import Frame from './stack-trace-frame.svelte';
import FrameHeader from './stack-trace-frame-header.svelte';
import FrameTitle from './stack-trace-frame-title.svelte';
import FrameLocation from './stack-trace-frame-location.svelte';
import FrameSourceButton from './stack-trace-frame-source-button.svelte';
import FrameSource from './stack-trace-frame-source.svelte';
import FrameContent from './stack-trace-frame-content.svelte';
import CopyButton from './stack-trace-copy-button.svelte';

export {
	Root,
	Header,
	Error,
	ErrorType,
	ErrorMessage,
	Frames,
	Frame,
	FrameHeader,
	FrameTitle,
	FrameLocation,
	FrameSourceButton,
	FrameSource,
	FrameContent,
	CopyButton,
	//
	Root as StackTrace,
	Header as StackTraceHeader,
	Error as StackTraceError,
	ErrorType as StackTraceErrorType,
	ErrorMessage as StackTraceErrorMessage,
	Frames as StackTraceFrames,
	Frame as StackTraceFrame,
	FrameHeader as StackTraceFrameHeader,
	FrameTitle as StackTraceFrameTitle,
	FrameLocation as StackTraceFrameLocation,
	FrameSourceButton as StackTraceFrameSourceButton,
	FrameSource as StackTraceFrameSource,
	FrameContent as StackTraceFrameContent,
	CopyButton as StackTraceCopyButton,
};

export {
	StackTraceState,
	getStackTraceContext,
	setStackTraceContext,
} from './stack-trace-context.svelte.js';

export { parseStackTrace, type StackFrame, type ParsedStackTrace } from './parse-stack.js';
