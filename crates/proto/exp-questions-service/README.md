# Eurora Questions Service

This service provides a gRPC API for handling content-related questions using OpenAI's API. It supports questions about videos, articles, and PDF documents.

## Features

- Video question answering with transcript highlighting and image context
- Article question answering with content and highlighted text
- PDF document question answering with content and highlighted text

## Setup

1. Create a `.env` file in the root directory with the following:

```
OPENAI_API_KEY=your_openai_api_key
OPENAI_MODEL=gpt-4o
QUESTIONS_SERVICE_PORT=50051
```

2. Build the service:

```
cargo build --release
```

## Running the Service

```
cargo run --release
```

The service will start on the port specified in the `.env` file (default: 50051).

## API

The service implements the `QuestionsService` gRPC interface defined in `proto/questions_service.proto`.

### Methods

- `VideoQuestion`: Process questions about video content with transcript and frame context
- `ArticleQuestion`: Process questions about article content with highlighted text
- `PdfQuestion`: Process questions about PDF documents with highlighted text

## Integration

This service is designed to be used by the monolith service, which acts as a proxy for client applications.