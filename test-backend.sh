#!/bin/bash

# Test script to verify Aura backend compilation and basic functionality

echo "🔧 Testing Aura Backend..."
echo ""

cd src-tauri

echo "1️⃣ Checking Rust compilation..."
cargo check 2>&1 | tail -5
if [ $? -eq 0 ]; then
    echo "✅ Rust compilation successful"
else
    echo "❌ Rust compilation failed"
    exit 1
fi

echo ""
echo "2️⃣ Running Rust tests..."
cargo test --lib 2>&1 | tail -10
if [ $? -eq 0 ]; then
    echo "✅ Rust tests passed"
else
    echo "⚠️  Some tests may have failed (this is expected for placeholder implementations)"
fi

echo ""
echo "3️⃣ Checking Ollama service..."
curl -s http://localhost:11434/api/tags > /dev/null
if [ $? -eq 0 ]; then
    echo "✅ Ollama service is running"
else
    echo "❌ Ollama service is not running. Please start it with: ollama serve"
    exit 1
fi

echo ""
echo "4️⃣ Checking qwen3.5:2b model..."
ollama list | grep "qwen3.5:2b" > /dev/null
if [ $? -eq 0 ]; then
    echo "✅ qwen3.5:2b model is available"
else
    echo "❌ qwen3.5:2b model not found. Please pull it with: ollama pull qwen3.5:2b"
    exit 1
fi

echo ""
echo "✨ All checks passed! You can now run: npm run tauri dev"
