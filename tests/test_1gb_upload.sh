#!/bin/bash

# Script to test 1GB+ file upload with full verification
# This script demonstrates the upload capability with large files

set -e

echo "🚀 IronDrop Large File Upload Test"
echo "=================================="
echo

# Check if user wants to run the 1GB+ test
if [ "${ENABLE_1GB_TEST}" != "1" ]; then
    echo "To enable 1GB+ testing, run:"
    echo "  ENABLE_1GB_TEST=1 $0"
    echo
    echo "⚠️  Warning: This test will:"
    echo "   - Create a 1GB+ file using fallocate/dd"
    echo "   - Consume 1GB+ RAM during upload"
    echo "   - Take several minutes to complete"
    echo "   - Require ~3GB free disk space"
    echo
    exit 0
fi

echo "🔧 Setting up 1GB+ upload test..."
echo "This will take several minutes and use significant system resources."
read -p "Continue? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Test cancelled."
    exit 0
fi

echo
echo "📊 System check:"
echo "Available memory: $(free -h | grep '^Mem:' | awk '{print $7}')"
echo "Available disk space: $(df -h . | tail -1 | awk '{print $4}')"
echo

# Run the 1GB+ test
echo "🧪 Running 1GB+ upload test..."
cd .. && ENABLE_1GB_TEST=1 cargo test test_very_large_file_upload_1gb_plus -- --ignored --nocapture

echo
echo "✅ Large file upload test completed successfully!"
echo "The upload architecture can handle files larger than 1GB without corruption."