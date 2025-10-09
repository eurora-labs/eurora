# Firefox Extension Deployment

This document describes the automated deployment process for the Eurora Firefox extension.

## Overview

The Firefox extension is automatically built and deployed to the Firefox Add-ons store using GitHub Actions. The workflow handles:

- Building the extension from source
- Updating the manifest version
- Replacing the development extension ID with the production ID
- Creating a signed extension package
- Submitting to the Firefox Add-ons store
- Creating GitHub releases for tagged versions

## Workflow Triggers

The deployment workflow can be triggered in two ways:

### 1. Manual Dispatch

Navigate to Actions → Deploy Firefox Extension → Run workflow

**Parameters:**

- **channel**: Choose between `production` (listed on store) or `beta` (unlisted)
- **version**: Specify the version number (e.g., `0.1.0`)

### 2. Git Tags

Push a tag matching the pattern `extension/v*.*.*`:

```bash
git tag extension/v0.1.0
git push origin extension/v0.1.0
```

This will automatically trigger a production deployment and create a GitHub release.

## Required GitHub Secrets

Before using the workflow, configure these secrets in your repository:

### Repository Settings → Secrets and variables → Actions → New repository secret

1. **`FIREFOX_API_KEY`**
    - Your Firefox Add-ons API key (JWT issuer)
    - Obtain from: https://addons.mozilla.org/developers/addon/api/key/

2. **`FIREFOX_API_SECRET`**
    - Your Firefox Add-ons API secret (JWT secret)
    - Obtain from: https://addons.mozilla.org/developers/addon/api/key/

### How to Get Firefox API Credentials

1. Go to https://addons.mozilla.org/developers/addon/api/key/
2. Sign in with your Firefox account
3. Click "Generate new credentials"
4. Copy the **JWT issuer** (this is your `FIREFOX_API_KEY`)
5. Copy the **JWT secret** (this is your `FIREFOX_API_SECRET`)
6. Add both to your GitHub repository secrets

### Environment Configuration

The workflow uses a GitHub environment named `firefox-addons`. To set this up:

1. Go to Settings → Environments → New environment
2. Name it `firefox-addons`
3. (Optional) Add protection rules:
    - Required reviewers
    - Wait timer
    - Deployment branches (e.g., only `main`)

## Extension ID Management

The extension uses different IDs for development and production:

- **Development ID**: `dev@eurora-labs.com`
- **Production ID**: `{271903fe-1905-4636-b47f-6f0873dc97f8}`

The workflow automatically replaces the dev ID with the production ID when deploying to the production channel.

## Build Process

The workflow performs these steps:

1. **Checkout code** and pull LFS files
2. **Setup Node.js environment** using pnpm
3. **Install Protoc** and compile protobuf definitions
4. **Extract version** from input or git tag
5. **Update manifest.json** with the specified version
6. **Build extensions** using `pnpm build`
7. **Replace extension ID** (production only)
8. **Create zip file** of the Firefox extension
9. **Upload artifact** to GitHub Actions
10. **Sign and publish** to Firefox Add-ons store
11. **Create GitHub release** (for tag-triggered deployments)

## Deployment Channels

### Production (Listed)

- Visible in Firefox Add-ons store search
- Available to all users
- Requires Mozilla review
- Triggered by: production channel selection or git tags

### Beta (Unlisted)

- Not visible in store search
- Accessible only via direct link
- Faster review process
- Triggered by: beta channel selection

## Monitoring Deployments

1. **GitHub Actions**: Check the workflow run status in the Actions tab
2. **Artifacts**: Download the built extension zip from the workflow artifacts
3. **Firefox Add-ons Dashboard**: Monitor review status at https://addons.mozilla.org/developers/
4. **GitHub Releases**: View published releases in the Releases section

## Troubleshooting

### Build Failures

- Check that all dependencies are properly installed
- Verify protobuf compilation succeeds
- Ensure the manifest.json is valid

### Signing Failures

- Verify API credentials are correct and not expired
- Check that the extension ID matches your registered add-on
- Ensure the version number hasn't been used before

### Review Rejections

- Review Mozilla's add-on policies: https://extensionworkshop.com/documentation/publish/add-on-policies/
- Check the review notes in your Firefox Add-ons dashboard
- Make necessary changes and re-submit with a new version

## Manual Deployment (Fallback)

If automated deployment fails, you can deploy manually:

1. Build the extension:

    ```bash
    pnpm build
    ```

2. Update the manifest ID:

    ```bash
    cd extensions/firefox
    # Edit manifest.json and change the ID to {271903fe-1905-4636-b47f-6f0873dc97f8}
    ```

3. Create a zip file:

    ```bash
    zip -r firefox-extension.zip . -x "*.git*" -x "*.zip"
    ```

4. Upload to Firefox Add-ons:
    - Go to https://addons.mozilla.org/developers/
    - Navigate to your add-on
    - Click "Upload New Version"
    - Upload the zip file

## Version Management

Version numbers should follow semantic versioning (MAJOR.MINOR.PATCH):

- **MAJOR**: Breaking changes
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes (backward compatible)

Update the version in:

- The workflow input or git tag
- The workflow automatically updates `extensions/firefox/manifest.json`

## Security Notes

- Never commit API credentials to the repository
- Use GitHub secrets for all sensitive data
- Regularly rotate API credentials
- Review the extension code before each release
- Monitor for security vulnerabilities in dependencies

## Support

For issues with:

- **Workflow**: Check GitHub Actions logs and this documentation
- **Firefox Add-ons**: Contact Mozilla support or check their documentation
- **Extension bugs**: Create an issue in the repository

## References

- [Firefox Extension Workshop](https://extensionworkshop.com/)
- [web-ext Documentation](https://extensionworkshop.com/documentation/develop/web-ext-command-reference/)
- [Firefox Add-ons Policies](https://extensionworkshop.com/documentation/publish/add-on-policies/)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
