#!/bin/bash
# Lookup Disk I/O Benchmark
# Measures lookup performance and real disk I/O (IOReadRequest/IOWriteRequest)
# with a disk-backed storage engine, comparing standard buffered I/O (Std)
# against O_DIRECT (StdDirect, bypasses the OS page cache).
# Fixed 8-byte keys/values, 32MB in-memory cache, varying preloaded record
# counts (1M to 32M) so the dataset exceeds the cache and lookups must hit disk.

cd "$(dirname "$0")" || exit 1

CSV_FILE="/tmp/benchmark_lookup_disk_io.csv"

echo "=========================================="
echo "LOOKUP DISK I/O BENCHMARK"
echo "=========================================="
echo "Testing: 1M, 2M, 4M, 8M, 16M, 32M preloaded records"
echo "Key Size: 8 bytes, Values: 8 bytes"
echo "Cache: 32MB in-memory"
echo "Backends: Std (buffered), StdDirect (O_DIRECT)"
echo ""

rm -f "$CSV_FILE"
rm -rf /tmp/bftree_disk_bench
cargo build --release --features metrics-rt 2>&1 | grep -E "Compiling|Finished"

echo ""
echo "Starting benchmarks..."
echo ""

for record_count in 1m 2m 4m 8m 16m 32m; do
  echo "=== Std backend: $record_count preloaded records ==="
  env BENCHMARK_CSV_PATH="$CSV_FILE" SHUMAI_FILTER="lookup_disk_record_count_${record_count}" timeout 7200 ./target/release/bftree 2>&1 | grep -E "Benchmark completed|Throughput:"
  echo ""
done

for record_count in 1m 2m 4m 8m 16m 32m; do
  echo "=== StdDirect backend: $record_count preloaded records ==="
  env BENCHMARK_CSV_PATH="$CSV_FILE" SHUMAI_FILTER="lookup_disk_direct_record_count_${record_count}" timeout 7200 ./target/release/bftree 2>&1 | grep -E "Benchmark completed|Throughput:"
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
