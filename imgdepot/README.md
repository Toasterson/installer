# Image Depot

Image Depot is an OCI Distribution Spec compliant image registry server implemented in Rust using Axum and OpenDAL.

## Features

- Fully compliant with the [OCI Distribution Specification](https://github.com/opencontainers/distribution-spec/blob/main/spec.md)
- Supports multiple storage backends via [OpenDAL](https://github.com/apache/opendal)
- Configurable for different environments (development and production)
- Lightweight and efficient

## Getting Started

### Prerequisites

- Rust 1.70 or later
- For S3 storage: AWS credentials or compatible S3 service

### Installation

1. Clone the repository
2. Build the project:

```bash
cargo build --release
```

The binary will be available at `target/release/imgdepotd`.

### Configuration

Image Depot uses configuration files located in the `config` directory:

- `default.toml`: Default configuration values
- `dev.toml`: Development environment configuration (uses local filesystem)
- `production.toml`: Production environment configuration (uses S3)

You can select the environment by setting the `RUN_MODE` environment variable:

```bash
# For development mode (default)
export RUN_MODE=dev

# For production mode
export RUN_MODE=production
```

#### Development Configuration

The development configuration uses local filesystem storage:

```toml
[storage]
backend = "fs"
fs_root = "./data"
```

#### Production Configuration

The production configuration uses S3 storage:

```toml
[storage]
backend = "s3"
s3_bucket = "imgdepot"
s3_region = "us-east-1"
# s3_endpoint = "https://s3.example.com"  # Optional
# s3_access_key = "your-access-key"       # Optional
# s3_secret_key = "your-secret-key"       # Optional
```

You can also configure S3 credentials using environment variables:

```bash
export IMGDEPOT_STORAGE_S3_ACCESS_KEY=your-access-key
export IMGDEPOT_STORAGE_S3_SECRET_KEY=your-secret-key
```

### Running

Start the server:

```bash
./target/release/imgdepotd
```

By default, the server listens on port 8080. You can change this in the configuration.

## Usage

Image Depot implements the OCI Distribution Specification, so it's compatible with standard container tools:

### Using with Docker

```bash
# Tag an image
docker tag ubuntu:latest localhost:8080/ubuntu:latest

# Push an image
docker push localhost:8080/ubuntu:latest

# Pull an image
docker pull localhost:8080/ubuntu:latest
```

### Using with Podman

```bash
# Tag an image
podman tag ubuntu:latest localhost:8080/ubuntu:latest

# Push an image
podman push localhost:8080/ubuntu:latest

# Pull an image
podman pull localhost:8080/ubuntu:latest
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.