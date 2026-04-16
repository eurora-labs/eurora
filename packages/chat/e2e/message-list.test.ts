import { test, expect, type Page } from '@playwright/test';

const TEST_PATH = '/message-list';

function messages(page: Page) {
	return page.locator('[data-slot="message"]');
}
function userMessages(page: Page) {
	return page.locator('[data-slot="message"].is-user');
}
function assistantMessages(page: Page) {
	return page.locator('[data-slot="message"].is-assistant');
}

async function setupConversation(page: Page) {
	await page.evaluate(() => {
		const t = (window as any).__test;
		return t.setupThread('thread-1', [
			t.makeMessageNode('h1', 'human', 'Hello there'),
			t.makeMessageNode('a1', 'ai', 'Hi! How can I help?', { parentId: 'h1' }),
			t.makeMessageNode('h2', 'human', 'Tell me a joke', { parentId: 'a1' }),
			t.makeMessageNode('a2', 'ai', 'Why did the chicken cross the road?', {
				parentId: 'h2',
			}),
		]);
	});
}

async function setupWithBranches(page: Page) {
	await page.evaluate(() => {
		const t = (window as any).__test;
		return t.setupThread('thread-1', [
			t.makeMessageNode('h1', 'human', 'Hello', {
				children: [{} as any, {} as any],
				siblingIndex: 0,
			}),
			t.makeMessageNode('a1', 'ai', 'Response A', {
				parentId: 'h1',
				children: [{} as any, {} as any],
				siblingIndex: 0,
			}),
		]);
	});
}

test.describe('MessageList', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto(TEST_PATH);
		await page.waitForSelector('[data-testid="debug-panel"]');
	});

	test.describe('empty state', () => {
		test('shows default empty state when no messages', async ({ page }) => {
			await page.evaluate(() => (window as any).__test.setupEmptyThread('thread-1'));

			await expect(page.locator('[data-slot="empty"]')).toBeVisible();
			await expect(page.getByText('No messages yet')).toBeVisible();
		});

		test('shows nothing when no active thread', async ({ page }) => {
			await expect(messages(page)).toHaveCount(0);
		});
	});

	test.describe('message rendering', () => {
		test('renders user and assistant messages', async ({ page }) => {
			await setupConversation(page);

			await expect(messages(page)).toHaveCount(4);
			await expect(userMessages(page)).toHaveCount(2);
			await expect(assistantMessages(page)).toHaveCount(2);
		});

		test('displays message text content', async ({ page }) => {
			await setupConversation(page);

			await expect(page.locator('[data-message-id="h1"]')).toContainText('Hello there');
			await expect(page.locator('[data-message-id="a1"]')).toContainText(
				'Hi! How can I help?',
			);
		});

		test('user messages are right-aligned', async ({ page }) => {
			await setupConversation(page);

			const userMsg = userMessages(page).first();
			await expect(userMsg).toHaveClass(/ml-auto/);
		});

		test('assistant messages are left-aligned', async ({ page }) => {
			await setupConversation(page);

			const aiMsg = assistantMessages(page).first();
			await expect(aiMsg).not.toHaveClass(/ml-auto/);
		});
	});

	test.describe('copy action', () => {
		test('copy button appears on messages', async ({ page }) => {
			await setupConversation(page);

			const msg = page.locator('[data-message-id="a1"]');
			await msg.hover();
			await expect(msg.getByRole('button', { name: 'Copy' })).toBeVisible();
		});

		test('clicking copy fires onCopy with message content', async ({ page }) => {
			await setupConversation(page);

			const msg = page.locator('[data-message-id="a1"]');
			await msg.hover();
			await msg.getByRole('button', { name: 'Copy' }).click();

			await expect(page.getByTestId('copied-content')).toHaveText('Hi! How can I help?');
		});
	});

	test.describe('edit action', () => {
		test('edit button appears only on user messages', async ({ page }) => {
			await setupConversation(page);

			const userMsg = page.locator('[data-message-id="h1"]');
			await userMsg.hover();
			await expect(userMsg.getByRole('button', { name: 'Copy' })).toBeVisible();
			await expect(userMsg.getByRole('button', { name: 'Edit' })).toBeVisible();

			const aiMsg = page.locator('[data-message-id="a1"]');
			await aiMsg.hover();
			await expect(aiMsg.getByRole('button', { name: 'Copy' })).toBeVisible();
			await expect(aiMsg.getByRole('button', { name: 'Edit' })).not.toBeVisible();
		});

		test('clicking edit shows textarea with current text', async ({ page }) => {
			await setupConversation(page);

			const userMsg = page.locator('[data-message-id="h1"]');
			await userMsg.hover();
			await userMsg.getByRole('button', { name: 'Edit' }).click();

			const editTextarea = userMsg.locator('textarea');
			await expect(editTextarea).toBeVisible();
			await expect(editTextarea).toHaveValue('Hello there');
		});

		test('cancel edit returns to normal message view', async ({ page }) => {
			await setupConversation(page);

			const userMsg = page.locator('[data-message-id="h1"]');
			await userMsg.hover();
			await userMsg.getByRole('button', { name: 'Edit' }).click();

			await expect(userMsg.locator('textarea')).toBeVisible();

			await userMsg.getByRole('button', { name: 'Cancel' }).click();
			await expect(userMsg.locator('textarea')).not.toBeVisible();
			await expect(userMsg).toContainText('Hello there');
		});

		test('Escape key cancels edit', async ({ page }) => {
			await setupConversation(page);

			const userMsg = page.locator('[data-message-id="h1"]');
			await userMsg.hover();
			await userMsg.getByRole('button', { name: 'Edit' }).click();

			await userMsg.locator('textarea').press('Escape');

			await expect(userMsg.locator('textarea')).not.toBeVisible();
		});

		test('submitting edit via Enter fires onEdit', async ({ page }) => {
			await setupConversation(page);

			const userMsg = page.locator('[data-message-id="h1"]');
			await userMsg.hover();
			await userMsg.getByRole('button', { name: 'Edit' }).click();

			const editTextarea = userMsg.locator('textarea');
			await editTextarea.fill('Updated message');
			await editTextarea.press('Enter');

			await expect(page.getByTestId('edited-message-id')).toHaveText('h1');
			await expect(page.getByTestId('edited-text')).toHaveText('Updated message');
		});

		test('submitting edit via Send button fires onEdit', async ({ page }) => {
			await setupConversation(page);

			const userMsg = page.locator('[data-message-id="h1"]');
			await userMsg.hover();
			await userMsg.getByRole('button', { name: 'Edit' }).click();

			const editTextarea = userMsg.locator('textarea');
			await editTextarea.fill('Button edit');
			await userMsg.getByRole('button', { name: 'Send' }).click();

			await expect(page.getByTestId('edited-message-id')).toHaveText('h1');
			await expect(page.getByTestId('edited-text')).toHaveText('Button edit');
		});

		test('edit does not submit empty text', async ({ page }) => {
			await setupConversation(page);

			const userMsg = page.locator('[data-message-id="h1"]');
			await userMsg.hover();
			await userMsg.getByRole('button', { name: 'Edit' }).click();

			const editTextarea = userMsg.locator('textarea');
			await editTextarea.fill('');
			await editTextarea.press('Enter');

			// Edit should still be open since empty text was rejected
			await expect(editTextarea).toBeVisible();
			await expect(page.getByTestId('edited-message-id')).toHaveText('');
		});
	});

	test.describe('streaming', () => {
		test('shows shimmer for empty AI message during streaming', async ({ page }) => {
			await page.evaluate(() => {
				const t = (window as any).__test;
				const aiNode = t.makeMessageNode('streaming-ai', 'ai', '');
				return t.setupThread('thread-1', [
					t.makeMessageNode('h1', 'human', 'Hello'),
					aiNode,
				]);
			});

			await page.evaluate(() =>
				(window as any).__test.simulateStreaming('thread-1', 'streaming-ai'),
			);

			await expect(page.locator('[data-slot="shimmer"]')).toBeVisible();
			await expect(page.getByText('Thinking')).toBeVisible();
		});

		test('hides message actions during streaming', async ({ page }) => {
			await page.evaluate(() => {
				const t = (window as any).__test;
				return t.setupThread('thread-1', [
					t.makeMessageNode('h1', 'human', 'Hello'),
					t.makeMessageNode('streaming-ai', 'ai', 'Partial response'),
				]);
			});

			await page.evaluate(() =>
				(window as any).__test.simulateStreaming('thread-1', 'streaming-ai'),
			);

			const streamingMsg = page.locator('[data-message-id="streaming-ai"]');
			await streamingMsg.hover();
			await expect(streamingMsg.locator('[data-slot="message-actions"]')).not.toBeVisible();
		});

		test('shows actions after streaming ends', async ({ page }) => {
			await page.evaluate(() => {
				const t = (window as any).__test;
				return t.setupThread('thread-1', [
					t.makeMessageNode('h1', 'human', 'Hello'),
					t.makeMessageNode('ai-1', 'ai', 'Complete response'),
				]);
			});

			const aiMsg = page.locator('[data-message-id="ai-1"]');
			await aiMsg.hover();
			await expect(aiMsg.locator('[data-slot="message-actions"]')).toBeVisible();
		});

		test('hides user edit actions while any message is streaming', async ({ page }) => {
			await page.evaluate(() => {
				const t = (window as any).__test;
				return t.setupThread('thread-1', [
					t.makeMessageNode('h1', 'human', 'Hello'),
					t.makeMessageNode('streaming-ai', 'ai', ''),
				]);
			});

			await page.evaluate(() =>
				(window as any).__test.simulateStreaming('thread-1', 'streaming-ai'),
			);

			const userMsg = page.locator('[data-message-id="h1"]');
			await userMsg.hover();
			await expect(userMsg.locator('[data-slot="message-actions"]')).not.toBeVisible();
		});
	});

	test.describe('branch navigation', () => {
		test('shows branch navigation for messages with siblings', async ({ page }) => {
			await setupWithBranches(page);

			const userMsg = page.locator('[data-message-id="h1"]');
			await userMsg.hover();

			await expect(userMsg.getByText('1 / 2')).toBeVisible();
		});

		test('does not show branch navigation for single-branch messages', async ({ page }) => {
			await setupConversation(page);

			const userMsg = page.locator('[data-message-id="h1"]');
			await userMsg.hover();

			await expect(userMsg.getByText(/\d+ \/ \d+/)).not.toBeVisible();
		});
	});

	test.describe('reasoning blocks', () => {
		test('renders reasoning block on AI messages', async ({ page }) => {
			await page.evaluate(() => {
				const t = (window as any).__test;
				return t.setupThread('thread-1', [
					t.makeMessageNode('h1', 'human', 'Explain something'),
					t.makeReasoningNode('a1', 'Let me think about this...', 'Here is the answer.'),
				]);
			});

			await expect(page.locator('[data-slot="reasoning"]')).toBeVisible();
		});

		test('reasoning trigger toggles content visibility', async ({ page }) => {
			await page.evaluate(() => {
				const t = (window as any).__test;
				return t.setupThread('thread-1', [
					t.makeMessageNode('h1', 'human', 'Explain'),
					t.makeReasoningNode('a1', 'Deep thoughts here', 'The answer is 42.'),
				]);
			});

			const trigger = page.locator('[data-slot="reasoning-trigger"]');
			await expect(trigger).toBeVisible();

			// Click to open reasoning
			await trigger.click();
			await expect(page.locator('[data-slot="reasoning-content"]')).toBeVisible();

			// Click to close reasoning
			await trigger.click();
			await expect(page.locator('[data-slot="reasoning-content"]')).not.toBeVisible();
		});
	});

	test.describe('auto-scroll', () => {
		test.use({ viewport: { width: 800, height: 600 } });

		async function setupStreamingConversation(page: Page) {
			await page.evaluate(() => {
				const t = (window as any).__test;
				const nodes = [];
				for (let i = 0; i < 30; i++) {
					nodes.push(
						t.makeMessageNode(
							'msg-' + i,
							i % 2 === 0 ? 'human' : 'ai',
							('Message ' + (i + 1) + '. ').repeat(20),
							i > 0 ? { parentId: 'msg-' + (i - 1) } : {},
						),
					);
				}
				nodes.push(t.makeMessageNode('streaming-ai', 'ai', '', { parentId: 'msg-29' }));
				return t.setupThread('thread-1', nodes);
			});
			await page.evaluate(() =>
				(window as any).__test.simulateStreaming('thread-1', 'streaming-ai'),
			);
			await page.evaluate(() => {
				const el = document.querySelector('[data-slot="conversation-content"]')!;
				el.scrollTo({ top: el.scrollHeight, behavior: 'auto' });
			});
			await page.waitForTimeout(100);
		}

		async function getScrollInfo(page: Page) {
			return await page.evaluate(() => {
				const el = document.querySelector('[data-slot="conversation-content"]');
				if (!el) return { scrollTop: 0, scrollHeight: 0, clientHeight: 0 };
				return {
					scrollTop: el.scrollTop,
					scrollHeight: el.scrollHeight,
					clientHeight: el.clientHeight,
				};
			});
		}

		async function assertAtBottom(page: Page) {
			await page.waitForTimeout(300);
			const info = await getScrollInfo(page);
			const dist = info.scrollHeight - info.scrollTop - info.clientHeight;
			expect(dist).toBeLessThan(5);
		}

		async function scrollUpAndDisengage(page: Page) {
			const content = page.locator('[data-slot="conversation-content"]');
			await content.hover();
			await page.mouse.wheel(0, -800);
			await page.waitForTimeout(200);
			const info = await getScrollInfo(page);
			const dist = info.scrollHeight - info.scrollTop - info.clientHeight;
			expect(dist).toBeGreaterThan(100);
		}

		test('auto-scrolls to bottom as streaming content arrives', async ({ page }) => {
			await setupStreamingConversation(page);

			await page.evaluate(() =>
				(window as any).__test.appendStreamChunk(
					'thread-1',
					'streaming-ai',
					'Hello world! '.repeat(50),
				),
			);

			await assertAtBottom(page);
		});

		test('mouse wheel up stops auto-scroll during streaming', async ({ page }) => {
			await setupStreamingConversation(page);

			await page.evaluate(() =>
				(window as any).__test.appendStreamChunk(
					'thread-1',
					'streaming-ai',
					'Initial chunk. '.repeat(50),
				),
			);
			await page.waitForTimeout(300);

			const content = page.locator('[data-slot="conversation-content"]');
			await content.hover();
			await page.mouse.wheel(0, -800);
			await page.waitForTimeout(200);

			const scrollAfterWheel = await getScrollInfo(page);

			for (let i = 0; i < 5; i++) {
				await page.evaluate(
					(i) =>
						(window as any).__test.appendStreamChunk(
							'thread-1',
							'streaming-ai',
							`Chunk ${i}. `.repeat(30),
						),
					i,
				);
				await page.waitForTimeout(100);
			}
			await page.waitForTimeout(200);

			const scrollAfterChunks = await getScrollInfo(page);
			const scrollDelta = Math.abs(scrollAfterChunks.scrollTop - scrollAfterWheel.scrollTop);
			expect(scrollDelta).toBeLessThan(50);
		});

		test('scrolling down does not disengage auto-scroll', async ({ page }) => {
			await setupStreamingConversation(page);

			await page.evaluate(() =>
				(window as any).__test.appendStreamChunk(
					'thread-1',
					'streaming-ai',
					'Content. '.repeat(50),
				),
			);
			await page.waitForTimeout(300);

			const content = page.locator('[data-slot="conversation-content"]');
			await content.hover();
			await page.mouse.wheel(0, 200);
			await page.waitForTimeout(100);

			await page.evaluate(() =>
				(window as any).__test.appendStreamChunk(
					'thread-1',
					'streaming-ai',
					'More content. '.repeat(50),
				),
			);

			await assertAtBottom(page);
		});

		test('sending a new message re-engages auto-scroll', async ({ page }) => {
			await setupStreamingConversation(page);

			await page.evaluate(() =>
				(window as any).__test.appendStreamChunk(
					'thread-1',
					'streaming-ai',
					'Content. '.repeat(50),
				),
			);
			await page.waitForTimeout(300);

			await scrollUpAndDisengage(page);

			await page.evaluate(() => (window as any).__test.stopStreaming('thread-1'));

			const aiId = await page.evaluate(() =>
				(window as any).__test.sendFakeMessage('thread-1', 'New question'),
			);
			await assertAtBottom(page);

			await page.evaluate(
				(id: string) =>
					(window as any).__test.appendStreamChunk(
						'thread-1',
						id,
						'New response. '.repeat(50),
					),
				aiId,
			);

			await assertAtBottom(page);
		});

		test('scrolling to bottom programmatically re-engages auto-scroll', async ({ page }) => {
			await setupStreamingConversation(page);

			await page.evaluate(() =>
				(window as any).__test.appendStreamChunk(
					'thread-1',
					'streaming-ai',
					'Content. '.repeat(50),
				),
			);
			await page.waitForTimeout(300);

			await scrollUpAndDisengage(page);

			await page.evaluate(() => {
				const el = document.querySelector('[data-slot="conversation-content"]')!;
				el.scrollTo({ top: el.scrollHeight, behavior: 'instant' as ScrollBehavior });
			});
			await page.waitForTimeout(200);

			await page.evaluate(() =>
				(window as any).__test.appendStreamChunk(
					'thread-1',
					'streaming-ai',
					'After re-engage. '.repeat(50),
				),
			);

			await assertAtBottom(page);
		});

		test('scrolling back to bottom manually re-engages auto-scroll', async ({ page }) => {
			await setupStreamingConversation(page);

			await page.evaluate(() =>
				(window as any).__test.appendStreamChunk(
					'thread-1',
					'streaming-ai',
					'Content. '.repeat(50),
				),
			);
			await page.waitForTimeout(300);

			await scrollUpAndDisengage(page);

			await page.mouse.wheel(0, 10000);
			await page.waitForTimeout(200);

			await page.evaluate(() =>
				(window as any).__test.appendStreamChunk(
					'thread-1',
					'streaming-ai',
					'After manual scroll back. '.repeat(50),
				),
			);

			await assertAtBottom(page);
		});

		test('switching threads scrolls to bottom', async ({ page }) => {
			await setupStreamingConversation(page);
			await page.evaluate(() => (window as any).__test.stopStreaming('thread-1'));

			await scrollUpAndDisengage(page);

			await page.evaluate(() => {
				const t = (window as any).__test;
				const nodes = [];
				for (let i = 0; i < 30; i++) {
					nodes.push(
						t.makeMessageNode(
							't2-msg-' + i,
							i % 2 === 0 ? 'human' : 'ai',
							('Thread 2 Message ' + (i + 1) + '. ').repeat(20),
							i > 0 ? { parentId: 't2-msg-' + (i - 1) } : {},
						),
					);
				}
				return t.switchThread('thread-2', nodes);
			});

			await assertAtBottom(page);
		});

		test('rapid streaming chunks stay pinned to bottom', async ({ page }) => {
			await setupStreamingConversation(page);

			await page.evaluate(async () => {
				const t = (window as any).__test;
				for (let i = 0; i < 20; i++) {
					await t.appendStreamChunk(
						'thread-1',
						'streaming-ai',
						`Rapid chunk ${i}. `.repeat(10),
					);
				}
			});

			await assertAtBottom(page);
		});

		test('auto-scroll works after content grows to fill viewport', async ({ page }) => {
			await page.evaluate(() => {
				const t = (window as any).__test;
				return t.setupThread('thread-1', [
					t.makeMessageNode('h1', 'human', 'Short message'),
					t.makeMessageNode('streaming-ai', 'ai', '', { parentId: 'h1' }),
				]);
			});
			await page.evaluate(() =>
				(window as any).__test.simulateStreaming('thread-1', 'streaming-ai'),
			);
			await page.waitForTimeout(100);

			for (let i = 0; i < 15; i++) {
				await page.evaluate(
					(i) =>
						(window as any).__test.appendStreamChunk(
							'thread-1',
							'streaming-ai',
							`Growing content block ${i}. `.repeat(30),
						),
					i,
				);
			}

			await assertAtBottom(page);
		});
	});
});
