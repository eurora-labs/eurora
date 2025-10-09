# GitHub Workflows for Eurora

This directory contains GitHub Actions workflows for automating various tasks in the Eurora project.

## Workflows

### Publish

The `publish.yaml` workflow is responsible for building and publishing the desktop app. It can be triggered in several ways:

- **Manually**: Go to the Actions tab in GitHub, select the "Publish" workflow, and click "Run workflow". You can choose the channel (release or nightly) and the version bump type (patch, minor, major).
- **Scheduled**: Runs automatically every day at 3am.
- **From Nightly Build**: Triggered by the completion of the "Nightly build" workflow.

The workflow performs the following steps:

1. Builds the SvelteKit frontend
2. Builds the Tauri app for multiple platforms (macOS, Linux, Windows)
3. Signs the binaries (if signing keys are provided)
4. Uploads the artifacts to S3 (if AWS credentials are provided)
5. Creates a git tag for the release
6. Notifies the Eurora API of the new release (if API token is provided)

### Deploy Firefox Extension

The `deploy-firefox-extension.yml` workflow automates the build and deployment of the Firefox extension to the Firefox Add-ons store. See [FIREFOX_EXTENSION_DEPLOYMENT.md](FIREFOX_EXTENSION_DEPLOYMENT.md) for detailed documentation.

**Triggers:**

- **Manually**: Actions tab → Deploy Firefox Extension → Run workflow (choose channel and version)
- **Git tags**: Push tags matching `extension/v*.*.*` (e.g., `extension/v0.1.0`)

**Key features:**

- Builds extension from source using `pnpm build`
- Automatically updates manifest version
- Replaces dev extension ID with production ID
- Signs and submits to Firefox Add-ons store
- Creates GitHub releases for tagged versions

**Required secrets:**

- `FIREFOX_API_KEY`: Firefox Add-ons API key (JWT issuer)
- `FIREFOX_API_SECRET`: Firefox Add-ons API secret (JWT secret)

### Nightly Build

The `nightly-build.yml` workflow is a simple trigger for the Publish workflow with the "nightly" channel. It runs:

- Every day at 2am
- When manually triggered

### E2E Tests Webdriver

The `test-e2e-webdriver.yml` workflow runs end-to-end tests using WebdriverIO. It is triggered:

- On pull requests to the main branch
- When manually triggered

## GitHub Actions

### init-env-node

This action sets up the Node.js environment for the workflows, including:

- Installing pnpm
- Setting up Node.js with the version specified in .nvmrc
- Installing dependencies

## Secrets Required

For full functionality, the following secrets should be configured in your GitHub repository:

### Desktop App (Publish workflow)

- `GITHUB_TOKEN`: For GitHub API access (automatically provided by GitHub)
- `TAURI_PRIVATE_KEY`: For signing Tauri updates
- `TAURI_KEY_PASSWORD`: Password for the Tauri private key
- `APPLE_CERTIFICATE`: Base64-encoded Apple certificate for macOS signing
- `APPLE_CERTIFICATE_PASSWORD`: Password for the Apple certificate
- `APPLE_ID`: Apple ID for notarization
- `APPLE_PROVIDER_SHORT_NAME`: Apple Team ID
- `APPLE_PASSWORD`: App-specific password for the Apple ID
- `APPIMAGE_PRIVATE_KEY`: GPG private key for signing AppImage
- `APPIMAGE_KEY_ID`: GPG key ID
- `APPIMAGE_KEY_PASSPHRASE`: Passphrase for the GPG key
- `AWS_ACCESS_KEY_ID`: AWS access key for S3 uploads
- `AWS_SECRET_ACCESS_KEY`: AWS secret key for S3 uploads
- `SENTRY_AUTH_TOKEN`: Sentry auth token for error reporting
- `SENTRY_CRONS`: Sentry cron monitoring URL
- `BOT_AUTH_TOKEN`: Auth token for the Eurora API

### Firefox Extension (Deploy Firefox Extension workflow)

- `FIREFOX_API_KEY`: Firefox Add-ons API key (JWT issuer) - [Get credentials](https://addons.mozilla.org/developers/addon/api/key/)
- `FIREFOX_API_SECRET`: Firefox Add-ons API secret (JWT secret)

Note: Many of these secrets are optional. The workflows will skip steps that require missing secrets.
