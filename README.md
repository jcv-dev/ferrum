# Ferrum 🎵

A lightweight, self-hosted music streaming server built with Rust. Stream your local music library with a modern REST API and JWT authentication.

[![CI](https://github.com/yourusername/ferrum/workflows/CI/badge.svg)](https://github.com/yourusername/ferrum/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- 🎶 **Stream local music** - MP3, FLAC, OGG, WAV, M4A, AAC, and more
- 🔐 **JWT Authentication** - Multi-user support with secure token-based auth
- 👤 **User Management** - First user becomes admin, self-registration
- 🔍 **Search & Filter** - Search by title, artist, album, or genre
- 📄 **Pagination** - Efficient browsing of large libraries
- 🖼️ **Cover Art** - Extract and serve embedded album artwork
- 🐳 **Docker Ready** - Easy deployment with Docker Compose
- 📊 **Structured Logging** - JSON logs for production, pretty logs for development
- 🛡️ **Security First** - Path traversal protection, CORS configuration, input validation

## Quick Start

### Using Docker (Recommended)

```bash
# Clone the repository
git clone https://github.com/yourusername/ferrum.git
cd ferrum

# Create environment file
cp .env.example .env
# Edit .env and set JWT_SECRET to a secure random string

# Start the server
docker-compose up -d

# View logs
docker-compose logs -f
```

### From Source

```bash
# Prerequisites: Rust 1.75+
cargo --version

# Clone and build
git clone https://github.com/yourusername/ferrum.git
cd ferrum
cargo build --release

# Create music directory and add your files
mkdir -p music
# Copy your music files to ./music/

# Configure and run
cp .env.example .env
# Edit .env as needed
./target/release/ferrum
```

## Configuration

All configuration is done via environment variables. See [.env.example](.env.example) for all options.

| Variable | Default | Description |
|----------|---------|-------------|
| `HOST` | `0.0.0.0` | Server bind address |
| `PORT` | `8080` | Server port |
| `MUSIC_FOLDER` | `./music` | Path to your music library |
| `USERS_FILE` | `./data/users.json` | User data storage location |
| `JWT_SECRET` | (random) | Secret key for signing tokens (set in production!) |
| `JWT_EXPIRY_DAYS` | `7` | Token validity period |
| `LOG_LEVEL` | `info` | Logging level (trace, debug, info, warn, error) |
| `LOG_FORMAT` | `pretty` | Log format (pretty or json) |
| `CORS_ORIGINS` | `*` | Allowed CORS origins (comma-separated) |

## API Reference

### Authentication

#### Register a new user
```bash
curl -X POST http://localhost:8080/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username": "myuser", "password": "mypassword123"}'
```

Response:
```json
{
  "user": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "username": "myuser",
    "is_admin": true,
    "created_at": "2024-01-15T10:30:00Z"
  },
  "token": {
    "access_token": "eyJ...",
    "token_type": "Bearer",
    "expires_in": 604800
  }
}
```

> **Note**: The first registered user automatically becomes an admin.

#### Login
```bash
curl -X POST http://localhost:8080/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "myuser", "password": "mypassword123"}'
```

#### Get current user
```bash
curl http://localhost:8080/auth/me \
  -H "Authorization: Bearer <token>"
```

### Music Library

All music endpoints require authentication.

#### List songs
```bash
curl "http://localhost:8080/api/music/list?page=1&per_page=20" \
  -H "Authorization: Bearer <token>"
```

Query parameters:
- `q` - Search query (title, artist, album)
- `artist` - Filter by artist
- `album` - Filter by album
- `genre` - Filter by genre
- `page` - Page number (default: 1)
- `per_page` - Items per page (default: 50, max: 100)
- `sort` - Sort field: `title`, `artist`, `album`, `year`, `duration`
- `order` - Sort order: `asc`, `desc`

Response:
```json
{
  "items": [
    {
      "id": "a1b2c3d4e5f67890",
      "title": "Song Title",
      "artist": "Artist Name",
      "album": "Album Name",
      "duration": 240,
      "track_number": 1,
      "year": 2023,
      "genre": "Rock",
      "format": "flac",
      "file": "song.flac",
      "has_cover": true
    }
  ],
  "total": 150,
  "page": 1,
  "per_page": 20,
  "total_pages": 8,
  "has_next": true,
  "has_prev": false
}
```

#### Stream a song
```bash
curl "http://localhost:8080/api/music/stream/song.mp3" \
  -H "Authorization: Bearer <token>" \
  --output song.mp3
```

Supports HTTP range requests for seeking.

#### Get album cover
```bash
curl "http://localhost:8080/api/music/cover/song.mp3" \
  -H "Authorization: Bearer <token>" \
  --output cover.jpg
```

#### List artists
```bash
curl "http://localhost:8080/api/music/artists" \
  -H "Authorization: Bearer <token>"
```

#### List albums
```bash
curl "http://localhost:8080/api/music/albums" \
  -H "Authorization: Bearer <token>"
```

### Health Checks

```bash
# Liveness check
curl http://localhost:8080/health

# Readiness check (verifies music folder is accessible)
curl http://localhost:8080/ready
```

## Project Structure

```
ferrum/
├── src/
│   ├── main.rs           # Application entry point
│   ├── config.rs         # Configuration management
│   ├── error.rs          # Error types and handling
│   ├── models.rs         # Data models
│   ├── auth/
│   │   ├── mod.rs
│   │   ├── jwt.rs        # JWT token handling
│   │   ├── middleware.rs # Auth extractors
│   │   └── user_repository.rs  # User storage
│   └── api/
│       ├── mod.rs
│       ├── auth.rs       # Auth endpoints
│       ├── health.rs     # Health endpoints
│       └── music.rs      # Music endpoints
├── Cargo.toml
├── Dockerfile
├── docker-compose.yml
└── .env.example
```

## Supported Audio Formats

- MP3 (`.mp3`)
- FLAC (`.flac`)
- OGG Vorbis (`.ogg`)
- WAV (`.wav`)
- AAC/M4A (`.m4a`, `.aac`)
- WMA (`.wma`)
- Opus (`.opus`)
- AIFF (`.aiff`)
- APE (`.ape`)

## Development

```bash
# Run with hot reload (requires cargo-watch)
cargo install cargo-watch
cargo watch -x run

# Run tests
cargo test

# Run clippy lints
cargo clippy -- -D warnings

# Format code
cargo fmt
```

## Building for Production

```bash
# Build optimized release binary
cargo build --release

# Binary will be at ./target/release/ferrum
```

The release build includes:
- LTO (Link Time Optimization)
- Single codegen unit for better optimization
- Stripped debug symbols
- Abort on panic (smaller binary)

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request