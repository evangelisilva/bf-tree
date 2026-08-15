#!/bin/bash
# Lookup Zipfian(1.0) Skewed Distribution Benchmark (StdDirect / O_DIRECT backend)
# Measures lookup performance with skewed (non-uniform) access pattern
# 1M key preload + 10K warmup + 10K timed lookups per key size
# Uses the StdDirect (O_DIRECT) storage backend, bypassing the OS page cache,
# so results include real disk I/O and page-read/write counts.

cd "$(dirname "$0")" || exit 1

CSV_FILE="/tmp/benchmark_lookup_zipf_skewed.csv"

echo "=========================================="
echo "LOOKUP ZIPFIAN(1.0) - SKEWED DISTRIBUTION (StdDirect)"
echo "=========================================="
echo "Testing: 8, 16, 32, 64, 128-byte keys"
echo "Preload: 1,000,000 keys (unmetered)"
echo "Warmup: 10,000 operations (unmetered)"
echo "Benchmark: 10,000 lookups (timed)"
echo "Distribution: Zipfian(1.0) - skewed"
echo "Cache: 32MB, Backend: StdDirect (O_DIRECT)"
echo "Output: $CSV_FILE"
echo ""

rm -f "$CSV_FILE"
rm -rf /tmp/bftree_disk_bench
cargo build --release --features metrics-rt 2>&1 | grep -E "Compiling|Finished"

echo ""
echo "Starting benchmarks..."
echo ""

for key_size in 8 16 32 64 128; do
  echo "=== $key_size-byte key benchmark ==="
  start_time=$(date +%s)
  
  env BENCHMARK_CSV_PATH="$CSV_FILE" SHUMAI_FILTER="lookup_${key_size}byte" timeout 600 ./target/release/bftree 2>&1 | tail -5
  
  end_time=$(date +%s)
  elapsed=$((end_time - start_time))
  echo "Elapsed: ${elapsed}s"
  echo ""
done

echo "=========================================="
echo "FINAL RESULTS"
echo "=========================================="
if [ -f "$CSV_FILE" ]; then
  cat "$CSV_FILE"
else
  echo "No results file found"
fi
