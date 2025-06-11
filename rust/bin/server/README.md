# Graphiti Server (Rust)

A Rust HTTP API server for the Graphiti temporal graph database.

## Features

- **REST API**: Compatible with the Python Graphiti server API
- **Episode Management**: Add episodes and messages to the temporal graph  
- **Search**: Search for relevant facts and relationships
- **Entity Management**: Create and manage entity nodes and edges
- **Group Operations**: Organize data by groups

## Configuration

The server is configured via environment variables:

```bash
# Required
OPENAI_API_KEY=your_openai_api_key
NEO4J_URI=bolt://localhost:7687
NEO4J_USER=neo4j
NEO4J_PASSWORD=your_password

# Optional
OPENAI_BASE_URL=https://api.openai.com/v1
MODEL_NAME=gpt-4
EMBEDDING_MODEL_NAME=text-embedding-ada-002
HOST=0.0.0.0
PORT=8000
```

## Building

```bash
# Build the server
cargo build --release --bin graphiti-server

# Run the server
cargo run --bin graphiti-server
```

## API Endpoints

### Health Check
- `GET /healthcheck` - Server health status

### Ingest Operations
- `POST /api/ingest/messages` - Add messages to the graph
- `POST /api/ingest/entity-node` - Create an entity node
- `DELETE /api/ingest/entity-edge/{uuid}` - Delete an entity edge
- `DELETE /api/ingest/group/{group_id}` - Delete a group
- `DELETE /api/ingest/episode/{uuid}` - Delete an episode

### Retrieve Operations  
- `POST /api/retrieve/search` - Search for facts
- `GET /api/retrieve/entity-edge/{uuid}` - Get entity edge by UUID
- `GET /api/retrieve/episodes/{group_id}` - Get episodes for a group
- `POST /api/retrieve/get-memory` - Get memory from messages

## Status

🚧 **Work in Progress** - This is a partial conversion from the Python server. Some endpoints are stubs and need implementation in the graphiti-core crate.

### Implemented
- ✅ Basic server structure with Axum
- ✅ Configuration management
- ✅ Episode creation via `add_episode`
- ✅ Search functionality
- ✅ Health check endpoint

### TODO
- 🔄 Complete entity node CRUD operations
- 🔄 Episode retrieval and deletion
- 🔄 Group management operations  
- 🔄 Async worker queue for message processing
- 🔄 Error handling and validation
- 🔄 Group filtering in search
- 🔄 Complete fact result metadata
