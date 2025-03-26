import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import type { Conversation, ChatMessage } from '@eurora/proto';

// Store for the list of conversations
export const conversations = writable<Conversation[]>([]);

// Store for the current conversation
export const currentConversation = writable<Conversation | null>(null);

// Load all conversations
export async function loadConversations() {
	try {
		const result = await invoke<Conversation[]>('list_conversations');
		conversations.set(result);
		return result;
	} catch (error) {
		console.error('Failed to load conversations:', error);
		return [];
	}
}

// Get a specific conversation
export async function getConversation(id: string) {
	try {
		const conversation = await invoke<Conversation>('get_conversation', { id });
		currentConversation.set(conversation);
		return conversation;
	} catch (error) {
		console.error(`Failed to get conversation ${id}:`, error);
		return null;
	}
}

// Create a new conversation
export async function createConversation(title: string) {
	try {
		const conversation = await invoke<Conversation>('create_conversation', { title });
		conversations.update((convs) => [conversation, ...convs]);
		currentConversation.set(conversation);
		return conversation;
	} catch (error) {
		console.error('Failed to create conversation:', error);
		return null;
	}
}

// Add a message to a conversation
export async function addMessage(id: string, role: string, content: string) {
	try {
		const updated = await invoke<Conversation>('add_conversation_message', {
			id,
			role,
			content
		});

		// Update both stores
		currentConversation.set(updated);
		conversations.update((convs) => convs.map((c) => (c.id === updated.id ? updated : c)));

		return updated;
	} catch (error) {
		console.error('Failed to add message:', error);
		return null;
	}
}

// Delete a conversation
export async function deleteConversation(id: string) {
	try {
		await invoke('delete_conversation', { id });

		conversations.update((convs) => convs.filter((c) => c.id !== id));
		currentConversation.update((current) => (current && current.id === id ? null : current));

		return true;
	} catch (error) {
		console.error('Failed to delete conversation:', error);
		return false;
	}
}
