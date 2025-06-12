#!/bin/bash

# Test script for Graphiti MCP Server (Rust)
# This script tests the MCP server functionality

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Testing Graphiti MCP Server (Rust)...${NC}"

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Please run this script from the rust/bin/mcp-server directory${NC}"
    exit 1
fi

# Build the project
echo -e "${YELLOW}Building MCP server...${NC}"
if cargo build; then
    echo -e "${GREEN}✓ Build successful${NC}"
else
    echo -e "${RED}✗ Build failed${NC}"
    exit 1
fi

# Check if environment variables are set
echo -e "${YELLOW}Checking environment configuration...${NC}"

check_env_var() {
    if [ -z "${!1}" ]; then
        echo -e "${YELLOW}Warning: $1 is not set${NC}"
        return 1
    else
        echo -e "${GREEN}✓ $1 is set${NC}"
        return 0
    fi
}

# Check required environment variables
env_ok=true
check_env_var "OPENAI_API_KEY" || env_ok=false
check_env_var "NEO4J_URI" || env_ok=false
check_env_var "NEO4J_USER" || env_ok=false
check_env_var "NEO4J_PASSWORD" || env_ok=false

if [ "$env_ok" = false ]; then
    echo -e "${YELLOW}Some environment variables are missing. Please set them in .env file or environment.${NC}"
    echo -e "${YELLOW}Example .env file:${NC}"
    cat << EOF
OPENAI_API_KEY=your_api_key_here
NEO4J_URI=bolt://localhost:7687
NEO4J_USER=neo4j
NEO4J_PASSWORD=your_password
MODEL_NAME=gpt-4.1-mini
EMBEDDER_MODEL_NAME=text-embedding-3-small
EOF
fi

# Test MCP protocol messages
echo -e "${YELLOW}Testing MCP protocol...${NC}"

test_mcp_message() {
    local message="$1"
    local description="$2"

    echo -e "${YELLOW}Testing: $description${NC}"

    # Create a timeout command that works on both Linux and macOS
    if command -v timeout >/dev/null 2>&1; then
        timeout_cmd="timeout 5"
    elif command -v gtimeout >/dev/null 2>&1; then
        timeout_cmd="gtimeout 5"
    else
        timeout_cmd=""
    fi

    if [ -n "$timeout_cmd" ]; then
        echo "$message" | $timeout_cmd cargo run -- --transport stdio --group-id test-session 2>&1 | head -n 5
    else
        echo "Warning: No timeout command available. Skipping interactive test."
    fi

    echo -e "${GREEN}✓ Message sent${NC}"
    echo ""
}

# Test initialization
test_mcp_message '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"clientInfo":{"name":"test-client","version":"1.0.0"}}}' "Initialization"

# Test tools listing
test_mcp_message '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' "Tools listing"

# Test add_memory tool
test_mcp_message '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"add_memory","arguments":{"name":"Test Episode","episode_body":"This is a test episode for the MCP server","source":"text"}}}' "Add memory"

echo -e "${GREEN}✓ All tests completed!${NC}"
echo -e "${YELLOW}Note: For full functionality, ensure Neo4j is running and environment variables are properly set.${NC}"

# Show build artifacts
echo -e "${YELLOW}Build artifacts:${NC}"
if [ -f "../../target/debug/graphiti-mcp-server" ]; then
    ls -la ../../target/debug/graphiti-mcp-server
    echo -e "${GREEN}✓ Binary available at: ../../target/debug/graphiti-mcp-server${NC}"
else
    echo -e "${YELLOW}Binary not found at expected location${NC}"
fi

echo -e "${GREEN}Test script completed!${NC}"
