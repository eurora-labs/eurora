# eur-openai

This crate provides OpenAI API integration for the Eurora project.

## Configuration

The crate uses a flexible configuration system that supports multiple ways to provide the OpenAI API key:

### 1. Environment Variables (Recommended for Development)

Copy the `.env.example` file to `.env` in the crate's directory and set your API key:

```bash
cp .env.example .env
```

Then edit the `.env` file and replace `your-api-key-here` with your actual OpenAI API key:

```
EUR_OPENAI_API_KEY=your-actual-api-key
```

### 2. Configuration Files

The crate also supports configuration through files in the `config` directory:

- `config/default.toml` - Default configuration
- `config/local.toml` - Local overrides (git-ignored)

Example configuration file:

```toml
openai_api_key = "your-api-key-here"
```

### 3. Environment Variables (Production)

For production environments, set the environment variable directly:

```bash
export EUR_OPENAI_API_KEY=your-api-key
```

## Configuration Priority

The configuration system follows this priority order (highest to lowest):

1. Environment variables with `EUR_` prefix
2. `config/local.toml`
3. `config/default.toml`
4. Default values

## Security Best Practices

1. Never commit API keys or sensitive data to version control
2. Add `.env` and `config/local.toml` to `.gitignore`
3. Use environment variables for production deployments
4. Rotate API keys periodically
5. Use restricted API keys with minimum required permissions

## Error Handling

The crate will return a `ConfigError` if:
- The configuration fails to load
- The OpenAI API key is missing or empty

Make sure to handle these errors appropriately in your application.