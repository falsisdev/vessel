# Vessel

Vessel is an umbrella, local-first, modular digital media consumption platform.

## Repository Layout

```
.
├── core/                       # Go Core application runtime
│   ├── cmd/core/               # Core daemon / CLI entrypoint
│   ├── internal/
│   │   ├── domain/cinema/      # Cinema domain models
│   │   ├── plugin/             # Plugin gRPC client and registry manager
│   │   └── service/            # Cinema aggregation service
│   └── test/integration/       # End-to-end integration tests
├── proto/                      # Protocol Buffers definitions
│   ├── plugin/v1/plugin.proto  # Plugin v1 contract
│   └── gen/go/                 # Generated Go gRPC/Protobuf bindings
├── plugins/
│   └── examples/cinema-mock/   # Reference standalone Cinema plugin
├── docs/
│   ├── adr/                    # Architecture Decision Records
│   └── architecture/           # Architecture design documents
├── scripts/                    # Development, build, and test scripts
├── buf.yaml                    # Buf module configuration
└── buf.gen.yaml                # Buf generation configuration
```

## Prerequisites

- [Go](https://go.dev/) (1.22+)
- [Buf](https://buf.build/) (for Protobuf workflows)
- [Protoc](https://github.com/protocolbuffers/protobuf) with `protoc-gen-go` and `protoc-gen-go-grpc`

## Quick Start

### 1. Generate Protobuf Bindings

```bash
./scripts/buf-generate.sh
```

### 2. Run Tests

```bash
./scripts/test.sh
```

### 3. Run Example Cinema Plugin

In a separate terminal:
```bash
go run ./plugins/examples/cinema-mock -port 50051
```

### 4. Query with Vessel Core

```bash
go run ./core/cmd/core -plugin-addr 127.0.0.1:50051 -search Batman
```
