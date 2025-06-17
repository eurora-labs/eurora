export default interface MessageType {
	role: 'user' | 'system';
	content: string;
	sources?: string[];
}
