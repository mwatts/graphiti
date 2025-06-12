# Graphiti MCP Server (Rust)

A Model Context Protocol (MCP) implementation for Graphiti knowledge graph memory, written in Rust.

## Overview

This is a Rust implementation of the Graphiti MCP server that provides memory capabilities for AI agents through the Model Context Protocol. It offers the same core functionality as the Python version but with improved performance and lower resource usage.

## Features

- **Episode Management**: Add episodes (text, JSON, messages) to the knowledge graph
- **Search Operations**: Search for nodes and facts in the graph memory
- **Graph Management**: Clear and manage the knowledge graph
- **Multiple Transports**: Support for stdio and SSE transports
- **Configuration**: Flexible configuration through environment variables and CLI arguments

## Installation & Setup

### Prerequisites

- Rust 1.70+ 
- Neo4j database running and accessible
- OpenAI API key (or compatible endpoint)

### Environment Variables

Create a `.env` file with the following configuration:

```bash
# OpenAI Configuration
OPENAI_API_KEY=your_openai_api_key_here
MODEL_NAME=gpt-4.1-mini
SMALL_MODEL_NAME=gpt-4.1-nano
EMBEDDER_MODEL_NAME=text-embedding-3-small
LLM_TEMPERATURE=0.0

# Neo4j Configuration  
NEO4J_URI=bolt://localhost:7687
NEO4J_USER=neo4j
NEO4J_PASSWORD=your_neo4j_password

# Azure OpenAI (Optional)
# AZURE_OPENAI_ENDPOINT=https://your-endpoint.openai.azure.com/
# AZURE_OPENAI_DEPLOYMENT_NAME=your-deployment-name
# AZURE_OPENAI_API_VERSION=2024-02-15-preview
# AZURE_OPENAI_USE_MANAGED_IDENTITY=false
```

### Building

```bash
cd rust/bin/mcp-server
cargo build --release
```

### Running

**Stdio Transport (for MCP clients):**
```bash
cargo run -- --transport stdio --group-id my-session
```

**SSE Transport (for web interfaces):**
```bash
cargo run -- --transport sse --group-id my-session
```

## Configuration Options

| Option | Description | Default |
|--------|-------------|---------|
| `--group-id` | Namespace for organizing related data | Random UUID |
| `--transport` | Communication transport (stdio/sse) | sse |
| `--model` | LLM model name | gpt-4.1-mini |
| `--small-model` | Small LLM model name | gpt-4.1-nano |
| `--temperature` | LLM temperature (0.0-2.0) | 0.0 |
| `--use-custom-entities` | Enable entity extraction | false |
| `--destroy-graph` | Clear all data on startup | false |

## MCP Tools

The server provides these MCP tools:

### `add_memory`
Add an episode to the knowledge graph.

**Parameters:**
- `name` (string): Episode name
- `episode_body` (string): Content to store
- `group_id` (string, optional): Group identifier
- `source` (string): Source type (text/json/message)
- `source_description` (string, optional): Source description
- `uuid` (string, optional): Custom UUID

### `search_memory_nodes`
Search for nodes (entities) in the graph.

**Parameters:**
- `query` (string): Search query
- `group_ids` (array, optional): Group IDs to filter
- `max_nodes` (integer): Maximum results (default: 10)
- `center_node_uuid` (string, optional): Center search around node
- `entity` (string, optional): Entity type filter

### `search_memory_facts`
Search for facts (relationships) in the graph.

**Parameters:**
- `query` (string): Search query
- `group_ids` (array, optional): Group IDs to filter  
- `max_facts` (integer): Maximum results (default: 10)
- `center_node_uuid` (string, optional): Center search around node

### `clear_graph`
Clear all data from the graph and rebuild indices.

**Parameters:** None

## Client Integration

### Claude Desktop
Add to your Claude Desktop MCP configuration:

```json
{
  "mcpServers": {
    "graphiti-memory": {
      "command": "/path/to/graphiti-mcp-server",
      "args": ["--transport", "stdio", "--group-id", "claude-session"],
      "env": {
        "OPENAI_API_KEY": "your-key-here",
        "NEO4J_URI": "bolt://localhost:7687",
        "NEO4J_USER": "neo4j", 
        "NEO4J_PASSWORD": "your-password"
      }
    }
  }
}
```

### Continue.dev
Add to your Continue configuration:

```json
{
  "mcpServers": [
    {
      "name": "graphiti-memory",
      "command": "/path/to/graphiti-mcp-server",
      "args": ["--transport", "stdio"],
      "env": {
        "OPENAI_API_KEY": "your-key-here"
      }
    }
  ]
}
```

## Development Status

This is an initial Rust implementation with the following current status:

### ✅ Implemented
- Basic MCP protocol support (stdio transport)
- Episode addition functionality
- Configuration management
- Core project structure

### 🚧 In Progress
- Search functionality (placeholder implementation)
- Graph clearing operations
- SSE transport support

### 📋 Planned
- Full search implementation with Graphiti core
- Enhanced error handling and validation
- Performance optimizations
- Comprehensive testing suite
- Documentation improvements

## Performance

The Rust implementation provides:
- **Faster startup times** compared to Python
- **Lower memory usage** for concurrent operations
- **Better resource efficiency** for long-running processes
- **Native async/await** support throughout

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Submit a pull request

## License

Licensed under the Apache License, Version 2.0. See LICENSE file for details.
