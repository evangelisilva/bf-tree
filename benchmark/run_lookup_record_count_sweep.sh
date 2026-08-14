#!/bin/bash
# Lookup Record Count Sweep Benchmark
# Measures lookup performance scaling with preloaded record count
# Fixed 8-byte keys, 8-byte values, varying preloaded records: 1M, 2M, 4M, 8M, 16M, 32M
# Each config runs 10,000 lookups (uniform distribution) after preload + warmup

cd "$(dirname "$0")" || exit 1

CSV_FILE="/tmp/benchmark_lookup_record_count_sweep.csv"

echo "=========================================="
echo "LOOKUP RECORD COUNT SWEEP BENCHMARK"
echo "=========================================="
echo "Testing: 1M, 2M, 4M, 8M, 16M, 32M preloaded records"
echo "Key Size: 8 bytes"
echo "Values: 8 bytes"
echo "Lookups: 10,000 (uniform distribution)"
echo "Cache: 32MB in-memory"
echo ""

rm -f "$CSV_FILE"
cargo build --release 2>&1 | grep -E "Compiling|Finished"

echo ""
echo "Starting benchmarks..."
echo ""

for record_count in 1m 2m 4m 8m 16m 32m; do
  echo "=== $record_count preloaded records benchmark ==="
  env BENCHMARK_CSV_PATH="$CSV_FILE" SHUMAI_FILTER="lookup_record_count_${record_count}" timeout 7200 ./target/release/bftree 2>&1 | grep -E "Benchmark completed|Throughput:"
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
