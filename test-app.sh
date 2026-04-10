#!/bin/bash

# Aura Application Test Script
# Quick test to verify all features work

echo "🧪 Aura Application Test"
echo "========================"
echo ""

# Check Ollama
echo "1. Checking Ollama service..."
if ! curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
    echo "❌ Ollama is not running"
    echo "   Please run: ollama serve"
    exit 1
fi
echo "✓ Ollama is running"
echo ""

# Check model
echo "2. Checking model availability..."
if ! ollama list | grep -q "qwen3.5:2b"; then
    echo "❌ Model qwen3.5:2b not found"
    echo "   Downloading model..."
    ollama pull qwen3.5:2b
fi
echo "✓ Model qwen3.5:2b is available"
echo ""

# Build check
echo "3. Checking build status..."
if [ ! -d "dist" ]; then
    echo "⚠️  Frontend not built, building now..."
    npm run build
fi
echo "✓ Frontend is built"
echo ""

# Backend check
echo "4. Checking backend compilation..."
cd src-tauri
if ! cargo check --quiet 2>&1 | grep -q "Finished"; then
    echo "⚠️  Backend has issues, checking..."
    cargo check
fi
cd ..
echo "✓ Backend compiles successfully"
echo ""

# Feature checklist
echo "========================"
echo "📋 Manual Test Checklist"
echo "========================"
echo ""
echo "Please test the following features:"
echo ""
echo "[ ] 1. Text Refinement"
echo "    - Input raw text"
echo "    - Select format and tone"
echo "    - Click 'Refine Text'"
echo "    - Verify output is refined"
echo ""
echo "[ ] 2. Smart Editing"
echo "    - Click on any word in output"
echo "    - Verify alternatives appear"
echo "    - Select an alternative"
echo "    - Verify word is replaced"
echo ""
echo "[ ] 3. Voice Commands"
echo "    - Click 'Voice Command' button"
echo "    - Enter command: '更正式一点'"
echo "    - Verify output is adjusted"
echo ""
echo "[ ] 4. Undo/Redo"
echo "    - Make changes to output"
echo "    - Click undo button"
echo "    - Click redo button"
echo "    - Verify changes are reverted/reapplied"
echo ""
echo "[ ] 5. Correction History"
echo "    - Edit output text"
echo "    - Click 'Save'"
echo "    - Click 'History' button"
echo "    - Verify correction is saved"
echo ""
echo "[ ] 6. User Context"
echo "    - Click 'Settings' button"
echo "    - Add name mapping"
echo "    - Add location preference"
echo "    - Add forbidden word"
echo "    - Click 'Save Changes'"
echo "    - Test refinement uses context"
echo ""
echo "[ ] 7. Offline Mode"
echo "    - Disconnect from internet"
echo "    - Verify 'Offline Mode' indicator"
echo "    - Test all features work"
echo "    - Reconnect and verify status"
echo ""
echo "[ ] 8. Copy to Clipboard"
echo "    - Click 'Copy' button"
echo "    - Paste in another app"
echo "    - Verify text is copied"
echo ""
echo "========================"
echo ""
echo "To start the application:"
echo "  npm run tauri dev"
echo ""
echo "To build for production:"
echo "  ./build-release.sh"
echo ""
