#!/usr/bin/env bash
set -e

BACKEND_PORT=${PORT:-8080}
FRONTEND_PORT=3001

cd app

if [ ! -d "node_modules" ]; then
  npm install
fi

# ----------------------------
# Start Backend
# ----------------------------
cd ../coinsmith
PORT=$BACKEND_PORT cargo run --release -- server &
BACKEND_PID=$!

# Give backend time to start
sleep 2

# ----------------------------
# Start Frontend
# ----------------------------
cd ../app
npm run dev -- --port $FRONTEND_PORT &
FRONTEND_PID=$!

echo "http://127.0.0.1:$FRONTEND_PORT"

# ----------------------------
# Cleanup
# ----------------------------
cleanup() {
  echo "Shutting down..."
  kill $BACKEND_PID 2>/dev/null || true
  kill $FRONTEND_PID 2>/dev/null || true
  wait
}

trap cleanup SIGINT SIGTERM
wait