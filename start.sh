#!/bin/bash

# Aura 启动脚本

echo "🌟 Starting Aura - Voice-to-Text Refinement Engine"
echo ""

# Check if Ollama is running
echo "📡 Checking Ollama service..."
if curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
    echo "✅ Ollama is running"
else
    echo "⚠️  Ollama is not running"
    echo "   Starting Ollama in background..."
    ollama serve > /dev/null 2>&1 &
    sleep 3
    
    if curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
        echo "✅ Ollama started successfully"
    else
        echo "❌ Failed to start Ollama"
        echo "   Please start it manually: ollama serve"
        exit 1
    fi
fi

# Check if model is available
echo ""
echo "🤖 Checking qwen3.5:2b model..."
if ollama list | grep -q "qwen3.5:2b"; then
    echo "✅ Model is available"
else
    echo "⚠️  Model not found"
    echo "   Downloading qwen3.5:2b (this may take a few minutes)..."
    ollama pull qwen3.5:2b
    
    if [ $? -eq 0 ]; then
        echo "✅ Model downloaded successfully"
    else
        echo "❌ Failed to download model"
        exit 1
    fi
fi

# Start Aura
echo ""
echo "🚀 Starting Aura application..."
echo ""
npm run tauri dev
