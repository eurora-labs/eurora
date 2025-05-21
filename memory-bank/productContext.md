# Product Context

This file provides a high-level overview of the project and the expected product that will be created. Initially it is based upon projectBrief.md (if provided) and all other available project-related information in the working directory. This file is intended to be updated as the project evolves, and should be used to inform all other modes of the project's goals and context.
2025-04-25 21:13:52 - Initial creation of Memory Bank.

## Project Goal

- Eurora appears to be an experimental AI-powered desktop application currently in early alpha testing.
- The project is structured as a monorepo containing multiple apps and packages.

## Key Features

- AI chat integration (packages/custom-components/ai-chat)
- Screen capture functionality (crates/app/eur-screen-capture)
- Conversation management (crates/app/eur-conversation)
- Timeline/focus tracking (crates/app/eur-timeline)
- OpenAI integration (crates/common/eur-openai)

## Overall Architecture

- Built using Tauri framework (Rust backend + web frontend)
- Monorepo structure with:
  - apps/ - Application frontends
  - crates/ - Rust backend code
  - packages/ - Frontend packages and components
  - proto/ - Protocol definitions
  - extensions/ - Browser extensions
  - scripts/ - Utility scripts
