# Eurora Monolith Service

This service acts as a central gRPC server that forwards requests to various microservices, currently focusing on the questions service.

## Features

- Single entry point for client applications
- Proxy for the questions service
- Support for video, article, and PDF document questions

## Setup

1. Create a `.env` file in the root directory with the following:

```
MONOLITH_PORT=50052
QUESTIONS_SERVICE_URL=http://[::1]:50051
```

2. Ensure the questions service is set up and running (see its README for instructions).

3. Build the monolith service:

```
cargo build --release
```

## Running the Service

```
cargo run --release
```

The service will start on the port specified in the `.env` file (default: 50052).

## Architecture

The monolith follows a service-oriented architecture:

1. Client applications connect to the monolith via gRPC
2. The monolith receives the request and forwards it to the appropriate microservice
3. The microservice processes the request and returns a response
4. The monolith forwards the response back to the client

## Integration

Client applications should connect to the monolith rather than directly to individual microservices. This allows for easier scaling, monitoring, and management of the system.

## Dependencies

- `eur-questions-service`: The questions service that handles content-related queries
- `eur-proto`: Shared Protobuf definitions and generated Rust code

## Development

To add a new microservice to the monolith:

1. Create a new gRPC service definition in the proto directory
2. Implement the service in a separate crate
3. Add a client connection in the monolith
4. Implement forwarding logic for the new service