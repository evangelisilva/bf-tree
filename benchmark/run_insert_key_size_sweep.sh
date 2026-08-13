#!/bin/bash
# Insert Key Size Sweep Benchmark
# Measures performance across different key sizes: 8, 16, 32, 64, 128 bytes
# Fixed at 1M records with 8-byte values

cd "$(dirname "$0")" || exit 1

CSV_FILE="/tmp/benchmark_insert_key_size_sweep.csv"

echo "=========================================="
echo "INSERT KEY SIZE SWEEP BENCHMARK"
echo "=========================================="
echo "Testing: 8, 16, 32, 64, 128-byte keys"
echo "Records: 1,000,000 per key size"
echo "Values: 8 bytes"
echo "Cache: 32MB in-memory"
echo ""

rm -f "$CSV_FILE"
cargo build --release 2>&1 | grep -E "Compiling|Finished"

echo ""
echo "Starting benchmarks..."
echo ""

for key_size in 8 16 32 64 128; do
  echo "=== $key_size-byte key benchmark ==="
  env BENCHMARK_CSV_PATH="$CSV_FILE" SHUMAI_FILTER="insert_key_size_sweep_${key_size}" timeout 3700 ./target/release/bftree 2>&1 | grep -E "Benchmark completed|Throughput:"
  echo ""
done

echo "=========================================="
echo "RESULTS"
echo "=========================================="
if [ -f "$CSV_FILE" ]; then
  cat "$CSV_FILE"
else
  echo "No results file found"
fi
