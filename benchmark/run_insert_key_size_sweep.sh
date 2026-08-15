#!/bin/bash
# Insert Key Size Sweep Benchmark (StdDirect / O_DIRECT backend)
# Measures performance across different key sizes: 8, 16, 32, 64, 128 bytes
# Fixed at 1M records with 8-byte values
# Uses the StdDirect (O_DIRECT) storage backend, bypassing the OS page cache,
# so results include real disk I/O and page-read/write counts.

cd "$(dirname "$0")" || exit 1

CSV_FILE="/tmp/benchmark_insert_key_size_sweep.csv"

echo "=========================================="
echo "INSERT KEY SIZE SWEEP BENCHMARK (StdDirect)"
echo "=========================================="
echo "Testing: 8, 16, 32, 64, 128-byte keys"
echo "Records: 1,000,000 per key size"
echo "Values: 8 bytes"
echo "Cache: 32MB, Backend: StdDirect (O_DIRECT)"
echo ""

rm -f "$CSV_FILE"
rm -rf /tmp/bftree_disk_bench
cargo build --release --features metrics-rt 2>&1 | grep -E "Compiling|Finished"

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
