import { TestRecorder } from './record.js';
import { spawn, type ChildProcess } from 'node:child_process';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Frameworks } from '@wdio/types';

const videoRecorder = new TestRecorder();
let tauriDriver: ChildProcess;

// Get the absolute path to the Tauri binary
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

export const config = {
	hostname: '127.0.0.1',
	runner: 'local',
	port: 4444,
	specs: ['./tests/**/*.spec.ts'],
	maxInstances: 1,
	capabilities: [
		{
			'tauri:options': {
				application: '../target/debug/eurora',
				// application: '../target/debug/euro-tauri',
			},
		},
	],
	reporters: ['spec'],
	framework: 'mocha',
	mochaOpts: {
		ui: 'bdd',
		timeout: 60000,
	},
	autoCompileOpts: {
		autoCompile: true,
		tsNodeOpts: {
			project: './tsconfig.json',
			transpileOnly: true,
		},
	},

	waitforTimeout: 10000,
	connectionRetryTimeout: 120000,
	connectionRetryCount: 0,

	beforeTest: async function (test: Frameworks.Test) {
		const videoPath = path.join(import.meta.dirname, 'videos');
		videoRecorder.start(test, videoPath);
	},

	afterTest: async function () {
		await sleep(2000); // Let browser settle before stopping.
		videoRecorder.stop();
	},

	// ensure we are running `tauri-driver` before the session starts so that we can proxy the webdriver requests
	beforeSession: () =>
		(tauriDriver = spawn(path.resolve(os.homedir(), '.cargo', 'bin', 'tauri-driver'), [], {
			stdio: [null, process.stdout, process.stderr],
		})),

	afterSession: () => {
		tauriDriver.kill();
	},
};

async function sleep(ms: number) {
	return await new Promise((resolve) => setTimeout(resolve, ms));
}
