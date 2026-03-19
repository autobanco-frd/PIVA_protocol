#!/bin/bash

# PIVA Project Check Script
# Runs all tests and provides clean output for development

echo "🔧 PIVA Project Health Check"
echo "================================"

# Clean previous builds
echo "🧹 Cleaning previous builds..."
cargo clean

# Check workspace compilation
echo "📦 Checking workspace compilation..."
if cargo check --workspace; then
    echo "✅ Workspace compiles successfully"
else
    echo "❌ Workspace compilation failed"
    exit 1
fi

# Run core tests
echo "🧪 Running piva-core tests..."
if cargo test -p piva-core; then
    echo "✅ piva-core tests passed"
else
    echo "❌ piva-core tests failed"
    exit 1
fi

# Run crypto tests
echo "🔐 Running piva-crypto tests..."
if cargo test -p piva-crypto; then
    echo "✅ piva-crypto tests passed"
else
    echo "❌ piva-crypto tests failed"
    exit 1
fi

# Run storage tests
echo "💾 Running piva-storage tests..."
if cargo test -p piva-storage; then
    echo "✅ piva-storage tests passed"
else
    echo "❌ piva-storage tests failed"
    exit 1
fi

# Run memory stress tests
echo "🧠 Running memory stress tests..."
if cargo test -p piva-core test_memory_stress_512mb; then
    echo "✅ Memory stress test passed"
else
    echo "❌ Memory stress test failed"
    exit 1
fi

if cargo test -p piva-core test_memory_stress_with_integrity_checks; then
    echo "✅ Memory stress with integrity checks passed"
else
    echo "❌ Memory stress with integrity checks failed"
    exit 1
fi

# Run integrity tests
echo "🔒 Running integrity tests..."
if cargo test -p piva-core test_verified_storage_workflow; then
    echo "✅ Verified storage workflow test passed"
else
    echo "❌ Verified storage workflow test failed"
    exit 1
fi

if cargo test -p piva-core test_integrity_resilience; then
    echo "✅ Integrity resilience test passed"
else
    echo "❌ Integrity resilience test failed"
    exit 1
fi

# Check total test count
echo "📊 Test Summary:"
echo "Total tests: $(cargo test --workspace --no-run --quiet 2>/dev/null | grep -o '[0-9]\+ tests' | head -1 || echo 'N/A')"

echo ""
echo "🎉 All PIVA tests passed successfully!"
echo "🚀 Ready for Sprint 5 - P2P Networking implementation"
