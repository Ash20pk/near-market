#!/bin/bash

# Test script to verify orderbook logging is working

echo "🧪 Testing orderbook service logging..."

# Clean up any existing logs
rm -f logs/orderbook.log*

# Start the orderbook service in background (non-TUI mode for testing)
echo "📋 Starting orderbook service (non-TUI mode)..."
RUST_LOG=info cargo run &
SERVICE_PID=$!

# Give the service time to start
sleep 3

# Check if log file was created
if [ -f "logs/orderbook.log" ]; then
    echo "✅ Log file created successfully"
    echo "📄 Recent log entries:"
    tail -n 10 logs/orderbook.log
else
    echo "❌ Log file not created"
fi

# Test with a simple health check
echo "🔍 Testing health check endpoint..."
curl -s http://localhost:8080/health > /dev/null
if [ $? -eq 0 ]; then
    echo "✅ Health check successful"
else
    echo "❌ Health check failed"
fi

# Kill the service
echo "🛑 Stopping orderbook service..."
kill $SERVICE_PID 2>/dev/null

# Wait a moment for cleanup
sleep 1

# Show final log content
if [ -f "logs/orderbook.log" ]; then
    echo "📋 Final log content:"
    cat logs/orderbook.log
else
    echo "❌ No log file found"
fi

echo "🏁 Logging test complete"