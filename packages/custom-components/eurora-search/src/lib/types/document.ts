/**
 * Interface for application information
 * Used by the application search providers
 */
export interface Document {
	/** Id of the document */
	id: string;

	/** Title of the app */
	title: string;

	/** Icon of the app */
	icon: string;
}
