import { test, expect } from '../utils/fixtures.ts';
import {
	cancelTool,
	invokeToolRaw,
	waitForBootstrap,
	waitForSiteMounted,
} from '../utils/helpers.ts';

test.describe('Tool framework error envelopes', { tag: '@default' }, () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('https://example.com/');
		await waitForBootstrap(page);
		await waitForSiteMounted(page, 'default');
	});

	test('INVOKE_TOOL with an unknown name returns remote/404', async ({ sw }) => {
		const reply = await invokeToolRaw(sw, 'totally_made_up_tool');
		expect(reply).toMatchObject({
			err: {
				kind: 'remote',
				code: 404,
			},
		});
	});

	test('INVOKE_TOOL with a malformed arg shape returns a decode error', async ({ sw }) => {
		/// `web_query_selector` requires `selector` to be a non-empty
		/// string; passing a number should be caught by the zod
		/// `argsSchema` parse inside `invokeFrom` before the handler
		/// runs, surfacing as a structured `decode` error instead of an
		/// adapter-level exception.
		const reply = await invokeToolRaw(sw, 'web_query_selector', { selector: 123 });
		expect(reply).toMatchObject({ err: { kind: 'decode' } });
	});

	test('INVOKE_TOOL rejects unknown keys via the strict arg schema', async ({ sw }) => {
		/// All tool arg schemas are declared `.strict()` — smuggling an
		/// unadvertised field through must also map to `decode` rather
		/// than being silently dropped.
		const reply = await invokeToolRaw(sw, 'web_get_page_metadata', { surprise: true });
		expect(reply).toMatchObject({ err: { kind: 'decode' } });
	});

	test('CANCEL_TOOL for an unknown call_id is a no-op', async ({ sw }) => {
		/// The framework guarantees `CANCEL_TOOL` is idempotent and
		/// resolves to `{}` regardless of whether the id is registered;
		/// this protects the desktop side from having to track which
		/// calls are still in-flight when the user hits cancel.
		await expect(cancelTool(sw, 999_999)).resolves.toBeUndefined();
	});
});
