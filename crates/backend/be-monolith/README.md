# be-monolith

The Eurora HTTP backend, served as a single Axum binary that merges all of
the `be-*-service` routers into one process. Auth, threads, activities,
assets, payment webhooks, and update artifact distribution all live behind
the same socket.

## Quickstart (local development)

From the repo root:

```sh
cp .env.example .env
$EDITOR .env       # set OPENAI_API_KEY
just dev           # brings up Postgres, seeds the dev user, runs the
                   # backend, the web auth UI, and the desktop app
```

That's it. The seeded `dev@dev.com` user (password `dev`) is preloaded
with a couple of threads so the desktop app has something to render after
login.

`just` recipes worth knowing:

- `just dev:backend` — Postgres + the backend binary, no web/desktop
- `just dev:reset` — wipe the DB volume and re-seed (use after schema
  migrations, or to regenerate dev data)
- `just dev:postgres` — Postgres only
- `just stop` — tear down docker-compose containers (volume preserved)

## Running natively without docker-compose

If you'd rather supply your own Postgres:

```sh
export REMOTE_DATABASE_URL=postgresql://...
export OPENAI_API_KEY=sk-...
export EURORA_CHAT_MODEL=gpt-4o-mini
cargo run -p be-monolith
```

The binary reads its LLM configuration from environment variables only —
no config file is required or supported. See "LLM provider configuration"
below for the full surface.

## Dev mode

`be-monolith` keys dev-mode behaviour off `cfg!(debug_assertions)`:

- Email service is not initialised; new users land on the database with
  `email_verified = true` so password registration works without SMTP.
- Stripe payment service is skipped; every user resolves to the `Tier1`
  role.
- Update service (S3-backed) is skipped.

There is no env-var override. `cargo run` is dev mode, `cargo build
--release` is production. This is intentional: it makes "is dev mode on?"
unambiguous from the build artefact.

## LLM provider configuration

Provider selection lives in `crates/common/llm-core` and is loaded by
`LlmConfig::from_env()` at startup. The env surface is:

| Variable               | Required for     | Notes                                                              |
| ---------------------- | ---------------- | ------------------------------------------------------------------ |
| `EURORA_LLM_KIND`      | —                | `openai` (default) or `openai_compatible`                          |
| `OPENAI_API_KEY`       | `openai`         |                                                                    |
| `EURORA_LLM_BASE_URL`  | `openai_compatible` (required); `openai` (optional override) | OpenAI-compatible servers must point this at e.g. `http://localhost:11434/v1` |
| `EURORA_LLM_API_KEY`   | `openai_compatible` (optional) | Many local servers don't authenticate                            |
| `EURORA_OPENAI_ORG`    | `openai` (optional) | Sent as `OpenAI-Organization`                                  |
| `EURORA_CHAT_MODEL`    | always           | Model name for chat                                                |
| `EURORA_TITLE_MODEL`   | optional         | Defaults to `EURORA_CHAT_MODEL`                                    |
| `EURORA_VISION_MODEL`  | optional         | When set, vision is enabled and bound to the same provider         |

Examples:

```sh
# OpenAI directly
OPENAI_API_KEY=sk-... EURORA_CHAT_MODEL=gpt-4o-mini cargo run -p be-monolith

# Local Ollama via its OpenAI shim
EURORA_LLM_KIND=openai_compatible \
EURORA_LLM_BASE_URL=http://localhost:11434/v1 \
EURORA_CHAT_MODEL=llama3.2 \
cargo run -p be-monolith

# OpenRouter
EURORA_LLM_KIND=openai_compatible \
EURORA_LLM_BASE_URL=https://openrouter.ai/api/v1 \
EURORA_LLM_API_KEY=sk-or-... \
EURORA_CHAT_MODEL=anthropic/claude-sonnet-4.5 \
cargo run -p be-monolith
```

`anthropic`, `google`, and `bedrock` are recognised in the `Provider`
schema but the runtime client wiring for those kinds isn't in place yet
— `EURORA_LLM_KIND=anthropic` returns a clear "not yet wired" error at
startup, and adding support means landing the relevant `agent-chain`
client and a match arm in `be-thread-service::llm::providers`.

## Inspecting the live config

The backend exposes a redacted view of the resolved configuration at
`GET /llm/info` (unauthenticated; never includes secrets):

```sh
curl http://localhost:3000/llm/info | jq
```

The desktop app's connection panel uses this same endpoint to render
"connected to: openai / gpt-4o-mini" before the user logs in.

## Pointing the desktop app at this backend

In **Settings → Connection** in the desktop app, pick:

- **Eurora Cloud** — `https://api.eurora-labs.com`
- **Local** — `http://localhost:3000` (what `just dev` brings up)
- **Custom** — any URL you self-host at

The "Test connection" button hits `/llm/info` on the chosen URL and
surfaces the active model in a toast — useful for confirming you're
talking to the right backend before persisting the change.
