#!/bin/bash

# Test script to verify the executable works when moved to different directories

set -e

echo "🧪 Testing executable portability with embedded templates..."

# Build the project
echo "📦 Building project..."
cargo build --release

# Create test directory structure
TEST_DIR=$(mktemp -d)
echo "📁 Created test directory: $TEST_DIR"

# Create some test files
echo "test file 1" > "$TEST_DIR/file1.txt"
echo "test file 2" > "$TEST_DIR/file2.txt"
mkdir -p "$TEST_DIR/subdir"
echo "nested file" > "$TEST_DIR/subdir/nested.txt"

# Copy executable to different locations
TEMP_EXE="/tmp/irondrop_portable_test"
cp target/release/irondrop "$TEMP_EXE"
chmod +x "$TEMP_EXE"

echo "✅ Executable copied to: $TEMP_EXE"

# Test 1: Run from /tmp directory
echo "🧪 Test 1: Running executable from /tmp..."
cd /tmp

# Start server in background
"$TEMP_EXE" -d "$TEST_DIR" -p 8082 --detailed-logging &
SERVER_PID=$!
sleep 2

# Test if server responds
if curl -s -f http://127.0.0.1:8082/ > /dev/null; then
    echo "✅ Test 1 PASSED: Server responds successfully from /tmp"
    
    # Test CSS asset
    if curl -s -f http://127.0.0.1:8082/_irondrop/static/directory/styles.css | grep -q "Professional Blackish Grey Design"; then
        echo "✅ CSS asset test PASSED: Embedded CSS is served correctly"
    else
        echo "❌ CSS asset test FAILED: Embedded CSS not working"
        kill $SERVER_PID 2>/dev/null || true
        exit 1
    fi
    
    # Test directory listing HTML
    if curl -s -f http://127.0.0.1:8082/ | grep -q "file1.txt"; then
        echo "✅ Directory listing test PASSED: Files are listed correctly"
    else
        echo "❌ Directory listing test FAILED: Files not listed"
        kill $SERVER_PID 2>/dev/null || true
        exit 1
    fi
    
    # Test 404 error page
    if curl -s http://127.0.0.1:8082/nonexistent | grep -q "404"; then
        echo "✅ Error page test PASSED: 404 page works correctly"
    else
        echo "❌ Error page test FAILED: 404 page not working"
        kill $SERVER_PID 2>/dev/null || true
        exit 1
    fi
    
else
    echo "❌ Test 1 FAILED: Server does not respond from /tmp"
    kill $SERVER_PID 2>/dev/null || true
    exit 1
fi

# Clean up
kill $SERVER_PID 2>/dev/null || true
sleep 1

# Test 2: Run from user's home directory
echo "🧪 Test 2: Running executable from home directory..."
cd "$HOME"

"$TEMP_EXE" -d "$TEST_DIR" -p 8083 --detailed-logging &
SERVER_PID=$!
sleep 2

if curl -s -f http://127.0.0.1:8083/ > /dev/null; then
    echo "✅ Test 2 PASSED: Server responds successfully from home directory"
else
    echo "❌ Test 2 FAILED: Server does not respond from home directory"
    kill $SERVER_PID 2>/dev/null || true
    exit 1
fi

# Clean up
kill $SERVER_PID 2>/dev/null || true
sleep 1

# Test 3: Run from a completely different directory
ANOTHER_DIR=$(mktemp -d)
echo "🧪 Test 3: Running executable from $ANOTHER_DIR..."
cd "$ANOTHER_DIR"

"$TEMP_EXE" -d "$TEST_DIR" -p 8084 --detailed-logging &
SERVER_PID=$!
sleep 2

if curl -s -f http://127.0.0.1:8084/ > /dev/null; then
    echo "✅ Test 3 PASSED: Server responds successfully from arbitrary directory"
    
    # Test that the web interface has proper styling
    if curl -s http://127.0.0.1:8084/ | grep -q "container"; then
        echo "✅ UI test PASSED: Web interface has proper styling"
    else
        echo "❌ UI test FAILED: Web interface missing styling"
        kill $SERVER_PID 2>/dev/null || true
        exit 1
    fi
    
else
    echo "❌ Test 3 FAILED: Server does not respond from arbitrary directory"
    kill $SERVER_PID 2>/dev/null || true
    exit 1
fi

# Clean up
kill $SERVER_PID 2>/dev/null || true
sleep 1

echo "🧪 Test 4: Reproducing the original issue scenario..."

# This should now work (whereas it previously failed)
cd /tmp
"$TEMP_EXE" -d "$HOME/Downloads" -p 8085 --detailed-logging &
SERVER_PID=$!
sleep 2

if curl -s -f http://127.0.0.1:8085/ > /dev/null; then
    echo "✅ Test 4 PASSED: Original issue scenario now works!"
else
    echo "❌ Test 4 FAILED: Original issue scenario still fails"
    kill $SERVER_PID 2>/dev/null || true
    exit 1
fi

# Clean up
kill $SERVER_PID 2>/dev/null || true

# Final cleanup
rm -f "$TEMP_EXE"
rm -rf "$TEST_DIR"
rm -rf "$ANOTHER_DIR"

echo "🎉 All tests PASSED! The executable is now fully portable with embedded templates."
echo "✨ The original issue has been resolved - templates are embedded and work from any directory."
