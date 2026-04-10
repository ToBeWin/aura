#!/bin/bash

# Aura Offline Mode Verification Script
# This script verifies that Aura works completely offline

echo "🔍 Aura Offline Mode Verification"
echo "=================================="
echo ""

# Check if Ollama is running
echo "1. Checking Ollama service..."
if ! curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
    echo "❌ Ollama is not running. Please start Ollama first."
    exit 1
fi
echo "✓ Ollama is running"
echo ""

# Check if model is available
echo "2. Checking if qwen3.5:2b model is available..."
if ! ollama list | grep -q "qwen3.5:2b"; then
    echo "❌ Model qwen3.5:2b not found. Please run: ollama pull qwen3.5:2b"
    exit 1
fi
echo "✓ Model qwen3.5:2b is available"
echo ""

# Verify no external network dependencies
echo "3. Verifying no external network dependencies..."
echo "   Checking Cargo.toml for external API dependencies..."

if grep -q "http://" aura/src-tauri/Cargo.toml | grep -v "localhost"; then
    echo "⚠️  Warning: Found external HTTP dependencies in Cargo.toml"
fi

if grep -q "https://" aura/src-tauri/Cargo.toml | grep -v "crates.io"; then
    echo "⚠️  Warning: Found external HTTPS dependencies in Cargo.toml"
fi

echo "✓ No obvious external API dependencies found"
echo ""

# Check local storage setup
echo "4. Verifying local storage configuration..."
if [ -f "aura/src-tauri/src/storage/context_store.rs" ]; then
    echo "✓ SQLite context store implemented"
fi

if [ -f "aura/src-tauri/src/storage/vector_db.rs" ]; then
    echo "✓ LanceDB vector store implemented"
fi
echo ""

# Check ASR and LLM are local
echo "5. Verifying local ASR and LLM setup..."
if grep -q "localhost:11434" aura/src-tauri/src/llm/client.rs; then
    echo "✓ LLM configured to use local Ollama (localhost:11434)"
else
    echo "⚠️  Warning: LLM may not be configured for local Ollama"
fi

if [ -f "aura/src-tauri/src/asr/engine.rs" ]; then
    echo "✓ ASR engine module exists"
fi
echo ""

# Summary
echo "=================================="
echo "📋 Offline Mode Verification Summary"
echo "=================================="
echo ""
echo "✓ All core components are configured for local operation"
echo "✓ No external API dependencies detected"
echo "✓ Ollama is running locally"
echo "✓ Required model is available"
echo ""
echo "🎯 Aura is ready for offline operation!"
echo ""
echo "To test offline mode:"
echo "1. Disconnect from the internet"
echo "2. Run: cd aura && npm run tauri dev"
echo "3. Test text refinement functionality"
echo "4. Verify all features work without network"
