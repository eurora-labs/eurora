import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import type { ChatMessage } from '@eurora/proto';

// Create a local store that mirrors the backend state
export const chatMessages = writable<ChatMessage[]>([]);

// Function to load messages from backend
export async function loadChatMessages() {
	try {
		const messages = await invoke<any[]>('get_chat_messages');
		chatMessages.set(
			messages.map((msg) => ({
				role: msg.role,
				content: msg.content
			}))
		);
		return messages;
	} catch (error) {
		console.error('Failed to load chat messages:', error);
		return [];
	}
}

// Function to add a message
export async function addChatMessage(role: 'user' | 'system', content: string) {
	try {
		await invoke('add_chat_message', { role, content });
		// Update local store
		chatMessages.update((messages) => [...messages, { role, content }]);
	} catch (error) {
		console.error('Failed to add chat message:', error);
	}
}

// Function to clear messages
export async function clearChatMessages() {
	try {
		await invoke('clear_chat_messages');
		chatMessages.set([]);
	} catch (error) {
		console.error('Failed to clear chat messages:', error);
	}
}

// Function to ask a question and get a response
export async function askQuestion(question: string): Promise<string> {
	try {
		// We don't need to manually add the user message here since
		// ask_video_question now handles storing both the question and answer
		const answer = await invoke<string>('ask_video_question', { question });

		// Refresh the message list
		await loadChatMessages();

		return answer;
	} catch (error) {
		console.error('Failed to get answer:', error);
		throw error;
	}
}
