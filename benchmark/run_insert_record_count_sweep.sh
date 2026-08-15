#!/bin/bash
# Insert Record Count Sweep Benchmark (StdDirect / O_DIRECT backend)
# Measures performance scaling with operation count
# Fixed 8-byte keys, varying operations: 1M, 2M, 4M, 8M, 16M, 32M
# Uses the StdDirect (O_DIRECT) storage backend, bypassing the OS page cache,
# so results include real disk I/O and page-read/write counts.

cd "$(dirname "$0")" || exit 1

CSV_FILE="/tmp/benchmark_insert_record_count_sweep.csv"

echo "=========================================="
echo "INSERT RECORD COUNT SWEEP BENCHMARK (StdDirect)"
echo "=========================================="
echo "Testing: 1M, 2M, 4M, 8M, 16M, 32M operations"
echo "Key Size: 8 bytes"
echo "Values: 8 bytes"
echo "Cache: 32MB, Backend: StdDirect (O_DIRECT)"
echo ""

rm -f "$CSV_FILE"
rm -rf /tmp/bftree_disk_bench
cargo build --release --features metrics-rt 2>&1 | grep -E "Compiling|Finished"

echo ""
echo "Starting benchmarks..."
echo ""

for op_count in 1m 2m 4m 8m 16m 32m; do
  echo "=== $op_count operation benchmark ==="
  env BENCHMARK_CSV_PATH="$CSV_FILE" SHUMAI_FILTER="insert_record_count_${op_count}" timeout 7200 ./target/release/bftree 2>&1 | grep -E "Benchmark completed|Throughput:"
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
