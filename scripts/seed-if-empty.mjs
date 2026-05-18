#!/usr/bin/env node
// Run the database seed only if the users table is empty. Idempotent
// first-boot path for `just dev`.
//
// Distinguishes three cases:
//   - schema absent  → bail with an actionable message (run `just dev-migrate`)
//   - schema present, users empty → run seed
//   - schema present, users present → skip
//
// `to_regclass('public.users')` is the schema-presence probe; it returns
// NULL without erroring if the table doesn't exist, which lets us tell
// "missing table" apart from a real psql failure.

import { run, runInherit } from './lib/proc.mjs';

// Postgres user/db are hardcoded in docker-compose.yml for the dev
// stack — they're conventions, not user config. We use the same
// values here so the host-side psql probe lines up with what the
// container was provisioned with. If you change them in compose,
// change them here too.
const PG_USER = 'postgres';
const PG_DB = 'eurora';

function psql(sql) {
	const r = run('docker', [
		'compose',
		'exec',
		'-T',
		'postgres',
		'psql',
		'-U',
		PG_USER,
		'-d',
		PG_DB,
		'-tAc',
		sql,
	]);
	if (r.code !== 0) {
		process.stderr.write(r.stderr);
		throw new Error(`psql failed (exit ${r.code}): ${sql}`);
	}
	return r.stdout.trim();
}

const schema = psql("SELECT to_regclass('public.users')");
if (schema === '') {
	console.error(
		"Schema not migrated yet. Run 'just dev-migrate' (or 'just dev', which does it automatically).",
	);
	process.exit(1);
}

const count = psql('SELECT count(*) FROM users');
if (count === '0') {
	console.log("Database is empty — running seed (creates dev@dev.com / password 'dev')");
	const code = await runInherit('docker', [
		'compose',
		'--profile',
		'seed',
		'up',
		'--no-deps',
		'--abort-on-container-exit',
		'seed',
	]);
	process.exit(code);
}

console.log(`Database already populated (${count} user(s)) — skipping seed.`);
