#!/bin/bash

# Basic test script for the Graphiti server
# This tests compilation and basic startup validation

echo "Testing Graphiti Server Build..."

# Test compilation
echo "1. Checking compilation..."
cargo check -p graphiti-server --quiet
if [ $? -eq 0 ]; then
    echo "✅ Compilation successful"
else
    echo "❌ Compilation failed"
    exit 1
fi

# Test build
echo "2. Building binary..."
cargo build -p graphiti-server --quiet
if [ $? -eq 0 ]; then
    echo "✅ Build successful"
else
    echo "❌ Build failed"
    exit 1
fi

# Check if binary exists
BINARY_PATH="target/debug/graphiti-server"
if [ -f "$BINARY_PATH" ]; then
    echo "✅ Binary created at $BINARY_PATH"
else
    echo "❌ Binary not found"
    exit 1
fi

echo ""
echo "🎉 Graphiti server build test completed successfully!"
echo ""
echo "To run the server:"
echo "1. Set up your environment variables (see .env.example)"
echo "2. Ensure Neo4j is running"
echo "3. Run: cargo run --bin graphiti-server"
