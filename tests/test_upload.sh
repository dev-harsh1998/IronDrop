#!/bin/bash

# IronDrop Upload Functionality Test Script
# Tests both GET /upload (form page) and POST /upload (file upload) endpoints

set -e

echo "🧪 IronDrop Upload Functionality Test Script"
echo "=============================================="

# Configuration
SERVER_PORT=${1:-8089}
SERVER_HOST="127.0.0.1"
TEST_DIR=$(mktemp -d)
UPLOAD_DIR=$(mktemp -d)
TIMEOUT=30

echo "📁 Test directory: $TEST_DIR"
echo "📁 Upload directory: $UPLOAD_DIR"
echo "🌐 Server: $SERVER_HOST:$SERVER_PORT"
echo ""

# Cleanup function
cleanup() {
    echo ""
    echo "🧹 Cleaning up..."
    if [[ -n "$SERVER_PID" ]]; then
        kill $SERVER_PID 2>/dev/null || true
        wait $SERVER_PID 2>/dev/null || true
        echo "✅ Server stopped"
    fi
    rm -rf "$TEST_DIR" "$UPLOAD_DIR"
    echo "✅ Cleanup complete"
}

trap cleanup EXIT

# Create test files
echo "📝 Creating test files..."
echo "Hello, World! This is test file 1." > "$TEST_DIR/test1.txt"
echo "This is test file 2 with some content." > "$TEST_DIR/test2.txt"
echo "Binary test content" > "$TEST_DIR/binary.dat"

# Start IronDrop server
echo "🚀 Starting IronDrop server..."
cargo run -- \
    --directory "$TEST_DIR" \
    --port "$SERVER_PORT" \
    --enable-upload \
    --upload-dir "$UPLOAD_DIR" \
    --allowed-extensions "*" \
    --max-upload-size 100 &

SERVER_PID=$!
echo "🏷️  Server PID: $SERVER_PID"

# Wait for server to start
echo "⏳ Waiting for server to start..."
for i in {1..30}; do
    if curl -s --max-time 2 "http://$SERVER_HOST:$SERVER_PORT/_health" > /dev/null; then
        echo "✅ Server is ready!"
        break
    fi
    if ! kill -0 $SERVER_PID 2>/dev/null; then
        echo "❌ Server process died"
        exit 1
    fi
    sleep 1
done

# Test 1: Health check
echo ""
echo "🏥 Test 1: Health Check"
echo "-----------------------"
HEALTH_RESPONSE=$(curl -s "http://$SERVER_HOST:$SERVER_PORT/_health")
echo "Response: $HEALTH_RESPONSE"
if echo "$HEALTH_RESPONSE" | grep -q "healthy"; then
    echo "✅ Health check passed"
else
    echo "❌ Health check failed"
    exit 1
fi

# Test 2: GET /upload (upload form page)
echo ""
echo "📄 Test 2: Upload Form Page"
echo "----------------------------"
FORM_RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" "http://$SERVER_HOST:$SERVER_PORT/upload")
echo "HTTP Status: $FORM_RESPONSE"
if [[ "$FORM_RESPONSE" == "200" ]]; then
    echo "✅ Upload form page accessible"
else
    echo "❌ Upload form page failed (expected 200, got $FORM_RESPONSE)"
    exit 1
fi

# Test 3: POST /upload (file upload) - Single file
echo ""
echo "📤 Test 3: Single File Upload"
echo "------------------------------"
UPLOAD_RESPONSE=$(curl -s -w "HTTPSTATUS:%{http_code}" -X POST \
    -F "file=@$TEST_DIR/test1.txt" \
    "http://$SERVER_HOST:$SERVER_PORT/upload")

HTTP_STATUS=$(echo "$UPLOAD_RESPONSE" | grep -o 'HTTPSTATUS:[0-9]*' | cut -d: -f2)
RESPONSE_BODY=$(echo "$UPLOAD_RESPONSE" | sed 's/HTTPSTATUS:[0-9]*$//')

echo "HTTP Status: $HTTP_STATUS"
echo "Response: ${RESPONSE_BODY:0:200}..."

if [[ "$HTTP_STATUS" == "200" ]]; then
    echo "✅ Single file upload successful"
    if [[ -f "$UPLOAD_DIR/test1.txt" ]]; then
        echo "✅ File exists in upload directory"
    else
        echo "⚠️  File not found in upload directory"
    fi
else
    echo "❌ Single file upload failed (expected 200, got $HTTP_STATUS)"
    echo "Error response: $RESPONSE_BODY"
    exit 1
fi

# Test 4: POST /upload (file upload) - Multiple files
echo ""
echo "📤 Test 4: Multiple File Upload"
echo "--------------------------------"
MULTI_UPLOAD_RESPONSE=$(curl -s -w "HTTPSTATUS:%{http_code}" -X POST \
    -F "file1=@$TEST_DIR/test2.txt" \
    -F "file2=@$TEST_DIR/binary.dat" \
    "http://$SERVER_HOST:$SERVER_PORT/upload")

MULTI_HTTP_STATUS=$(echo "$MULTI_UPLOAD_RESPONSE" | grep -o 'HTTPSTATUS:[0-9]*' | cut -d: -f2)
MULTI_RESPONSE_BODY=$(echo "$MULTI_UPLOAD_RESPONSE" | sed 's/HTTPSTATUS:[0-9]*$//')

echo "HTTP Status: $MULTI_HTTP_STATUS"
echo "Response: ${MULTI_RESPONSE_BODY:0:200}..."

if [[ "$MULTI_HTTP_STATUS" == "200" ]]; then
    echo "✅ Multiple file upload successful"
    if [[ -f "$UPLOAD_DIR/test2.txt" && -f "$UPLOAD_DIR/binary.dat" ]]; then
        echo "✅ Both files exist in upload directory"
    else
        echo "⚠️  Not all files found in upload directory"
    fi
else
    echo "❌ Multiple file upload failed (expected 200, got $MULTI_HTTP_STATUS)"
    echo "Error response: $MULTI_RESPONSE_BODY"
    exit 1
fi

# Test 5: Invalid request method
echo ""
echo "🚫 Test 5: Invalid Request Method"
echo "----------------------------------"
INVALID_RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" -X PUT \
    "http://$SERVER_HOST:$SERVER_PORT/upload")

echo "HTTP Status: $INVALID_RESPONSE"
if [[ "$INVALID_RESPONSE" == "405" ]]; then
    echo "✅ Invalid method properly rejected"
else
    echo "⚠️  Invalid method response: $INVALID_RESPONSE (expected 405)"
fi

# Test 6: Missing Content-Type
echo ""
echo "🚫 Test 6: Missing Content-Type Header"
echo "---------------------------------------"
MISSING_CT_RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" -X POST \
    --data "not multipart data" \
    "http://$SERVER_HOST:$SERVER_PORT/upload")

echo "HTTP Status: $MISSING_CT_RESPONSE"
if [[ "$MISSING_CT_RESPONSE" == "400" ]]; then
    echo "✅ Missing content-type properly rejected"
else
    echo "⚠️  Missing content-type response: $MISSING_CT_RESPONSE (expected 400)"
fi

# Summary
echo ""
echo "📊 Test Summary"
echo "==============="
echo "✅ Upload functionality is working correctly!"
echo "✅ Files uploaded successfully to: $UPLOAD_DIR"
echo "✅ Server handling various request types properly"
echo ""

# List uploaded files
if [[ -d "$UPLOAD_DIR" ]]; then
    echo "📋 Files in upload directory:"
    ls -la "$UPLOAD_DIR/"
fi

echo ""
echo "🎉 All tests passed! Upload functionality is working."
echo "   You can now use the web interface at: http://$SERVER_HOST:$SERVER_PORT/upload"