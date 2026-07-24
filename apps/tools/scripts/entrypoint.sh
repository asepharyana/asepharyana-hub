#!/bin/bash
# Tools Service Entrypoint
# Starts both the Gateway (Axum HTTP server) and Workers (NATS consumers)

set -e

echo "Starting tools-gateway..."
/app/gateway &
GATEWAY_PID=$!

sleep 1

echo "Starting tools-workers..."
/app/workers &
WORKER_PID=$!

# Handle graceful shutdown
trap "echo 'Shutting down...'; kill $GATEWAY_PID $WORKER_PID 2>/dev/null; exit 0" SIGINT SIGTERM

# Wait for either process to exit
wait -n $GATEWAY_PID $WORKER_PID

# If one exits, kill the other
kill $GATEWAY_PID $WORKER_PID 2>/dev/null
exit 1