#!/bin/bash
# Lookup Zipf Uniform Benchmark
# Measures performance with uniform access pattern (Zipf 0.0)
# Fixed 1M preloaded records, varying key sizes: 8, 16, 32, 64, 128 bytes

cd "$(dirname "$0")" || exit 1

CSV_FILE="/tmp/benchmark_lookup_zipf_uniform.csv"

echo "=========================================="
echo "LOOKUP ZIPF UNIFORM BENCHMARK"
echo "=========================================="
echo "Testing: 8, 16, 32, 64, 128-byte keys"
echo "Preloaded: 1,000,000 records"
echo "Distribution: Zipfian(0.0) - uniform"
echo "Values: 128 bytes"
echo "Cache: 32MB in-memory"
echo ""

rm -f "$CSV_FILE"
cargo build --release 2>&1 | grep -E "Compiling|Finished"

echo ""
echo "Starting benchmarks..."
echo ""

for key_size in 8 16 32 64 128; do
  echo "=== $key_size-byte key benchmark ==="
  env BENCHMARK_CSV_PATH="$CSV_FILE" SHUMAI_FILTER="lookup_zipf_uniform_${key_size}byte" timeout 3700 ./target/release/bftree 2>&1 | grep -E "Benchmark completed|Throughput:"
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
