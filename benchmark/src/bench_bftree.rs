// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

use std::sync::atomic::{AtomicUsize, Ordering};

use bf_tree::{
    circular_buffer::CircularBufferMetrics, metric::Timer, timer, BfTree, LeafReadResult,
    ScanReturnField,
};
use rand::{rngs::SmallRng, SeedableRng};
use shumai::{config, ShumaiBench};

use crate::{
    bench_e2e::install_value_to_buffer,
    common::{Distribution, MicroBenchResult, Sampler, StorageBackend, Workload, WorkloadMix},
};

#[config(path = "bench_bftree.toml")]
pub struct BfTreeBench {
    pub name: String,
    pub threads: Vec<usize>,
    pub time: usize,
    pub repeat: usize,
    pub record_cnt: usize,
    #[matrix]
    pub distribution: Distribution,
    #[matrix]
    pub workload_mix: WorkloadMix,
    #[matrix]
    pub memory_size_mb: usize,
    pub file_path: String,
    pub key_len: usize, // must be multiple of 8
    pub scan_cnt: usize,
    #[matrix]
    pub storage: StorageBackend,
    #[matrix]
    pub read_promotion_rate: u64,
    #[matrix]
    pub copy_on_access_ratio: f64,
    #[serde(default)]
    pub record_count_mode: bool,
    #[serde(default)]
    pub value_len: usize,
    #[serde(default)]
    pub warmup_cnt: usize,
    #[serde(default)]
    pub preload_records: usize,
}

struct TestBench {
    bftree: BfTree,
    config: BfTreeBench,
    positive_sampler: Sampler, // sample only records that are inserted in load phase
    all_sampler: Sampler,      // sample records from all possible records
}

impl TestBench {
    fn new(c: &BfTreeBench) -> Self {
        let memory_size = c.memory_size_mb * 1024 * 1024;
        assert!(
            c.key_len.is_multiple_of(8),
            "key_len must be multiple of 8!"
        );

        _ = std::fs::remove_file(&c.file_path);
        _ = std::fs::remove_dir_all(&c.file_path);

        // Use preload_records if set (for lookups), otherwise use record_cnt (for inserts)
        let preload_or_record_cnt = if c.preload_records > 0 {
            c.preload_records
        } else {
            c.record_cnt
        };

        let positive_sampler = Sampler::from(&c.distribution, 0..preload_or_record_cnt);
        let all_sampler = Sampler::from(&c.distribution, 0..usize::MAX);

        let mut config = bf_tree::Config::new(&c.file_path, memory_size);
        config.read_promotion_rate(c.read_promotion_rate as usize);
        config.storage_backend(c.storage.into());
        config.cb_copy_on_access_ratio(c.copy_on_access_ratio);
        let bf_tree = BfTree::with_config(config, None).unwrap();

        Self {
            bftree: bf_tree,
            config: c.clone(),
            positive_sampler,
            all_sampler,
        }
    }
}

impl ShumaiBench for TestBench {
    type Config = BfTreeBench;
    type Result = MicroBenchResult;

    fn run(&self, context: shumai::Context<Self::Config>) -> MicroBenchResult {
        let mut small_rng = SmallRng::from_os_rng();
        let mut key_buffer = vec![0; self.config.key_len / 8];
        let value_len = if self.config.value_len > 0 {
            self.config.value_len
        } else {
            self.config.key_len
        };
        let mut value_buffer_usize = vec![0usize; (value_len + 7) / 8];
        let mut value_buffer_u8: Vec<u8> = vec![0; value_len];
        let mut op_cnt = 0;

        bf_tree::metric::get_tls_recorder().reset();

        context.wait_for_start();

        // Warmup phase: perform warmup_cnt reads without timing
        if self.config.warmup_cnt > 0 {
            eprintln!("Running warmup: {} operations", self.config.warmup_cnt);
            for _ in 0..self.config.warmup_cnt {
                let key_id = self.positive_sampler.sample(&mut small_rng);
                let key = install_value_to_buffer(&mut key_buffer, key_id);
                
                let cnt = self.bftree.read(key, &mut value_buffer_u8);
                match cnt {
                    LeafReadResult::Found(v) => {
                        assert_eq!(v as usize, key.len());
                        assert_eq!(key, &value_buffer_u8[0..v as usize]);
                    }
                    _ => {
                        eprintln!("Missing key during warmup, marking benchmark as FAILED");
                        log_benchmark_result(&self.config.name, x_axis_value(&self.config), "FAILED", None);
                        std::process::exit(0);
                    }
                }
            }
            eprintln!("Warmup completed");
        }

        // Reset metrics before benchmark phase
        bf_tree::metric::get_tls_recorder().reset();

        let start_time = std::time::Instant::now();

        loop {
            // Check if we should exit based on record_count_mode
            if self.config.record_count_mode {
                if op_cnt >= self.config.record_cnt {
                    break;
                }
            } else {
                if !context.is_running() {
                    break;
                }
            }

            let op = self.config.workload_mix.gen(&mut small_rng);
            timer!(Timer::Read);
            match op {
                Workload::Read => {
                    let key_id = self.positive_sampler.sample(&mut small_rng);
                    let key = install_value_to_buffer(&mut key_buffer, key_id);

                    let cnt = self.bftree.read(key, &mut value_buffer_u8);
                    match cnt {
                        LeafReadResult::Found(v) => {
                            assert_eq!(v as usize, key.len());
                            assert_eq!(key, &value_buffer_u8[0..v as usize]);
                        }
                        _ => {
                            eprintln!("Missing key during benchmark, marking benchmark as FAILED");
                            log_benchmark_result(&self.config.name, x_axis_value(&self.config), "FAILED", None);
                            std::process::exit(0);
                        }
                    }
                    op_cnt += 1;
                }
                Workload::NegativeRead => {}
                Workload::Scan => {
                    let key_id = self.positive_sampler.sample(&mut small_rng);
                    let key = install_value_to_buffer(&mut key_buffer, key_id);

                    let mut iter = self
                        .bftree
                        .scan_with_count(key, self.config.scan_cnt, ScanReturnField::Value)
                        .expect("Failed to create scan iterator");

                    while let Some((key_len, value_len)) = iter.next(&mut value_buffer_u8) {
                        assert!(value_len <= value_buffer_u8.len());
                        assert!(key_len == 0);
                    }

                    op_cnt += 1;
                }
                Workload::Update => {
                    let key_id = self.positive_sampler.sample(&mut small_rng);
                    let key = install_value_to_buffer(&mut key_buffer, key_id);

                    self.bftree.insert(key, key);
                    op_cnt += 1;
                }
                Workload::Insert => {
                    let key_id = self.all_sampler.sample(&mut small_rng);
                    let key = install_value_to_buffer(&mut key_buffer, key_id);
                    let value = install_value_to_buffer(&mut value_buffer_usize, key_id);

                    self.bftree.insert(key, value);
                    op_cnt += 1;
                    
                    if self.config.record_count_mode && op_cnt % 100_000 == 0 {
                        eprintln!("Progress: {}/{}", op_cnt, self.config.record_cnt);
                    }
                }
            }
        }

        let elapsed = start_time.elapsed();
        let elapsed_secs = elapsed.as_secs_f64();
        eprintln!("Benchmark completed: {} ops in {:.2}s (throughput: {:.0} ops/s)", 
                  op_cnt, elapsed_secs, op_cnt as f64 / elapsed_secs);

        let metric = if cfg!(feature = "metrics-rt") {
            Some(bf_tree::metric::get_tls_recorder().clone())
        } else {
            None
        };

        // Extract IOReadRequest/IOWriteRequest and page-read counters from metrics
        // for CSV logging (only meaningful for disk-backed storage; Memory backend
        // never counts IO, but page reads are tracked regardless of backend)
        let (io_reads, io_writes, base_page_reads, full_page_reads, mini_page_reads) =
            if let Some(ref m) = metric {
                if let Ok(json_val) = serde_json::to_value(m) {
                    let counters = json_val.get("counters");
                    let get_counter = |name: &str| {
                        counters
                            .and_then(|c| c.get(name))
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as usize
                    };
                    (
                        get_counter("IOReadRequest"),
                        get_counter("IOWriteRequest"),
                        get_counter("BasePageRead"),
                        get_counter("FullPageRead"),
                        get_counter("MiniPageRead"),
                    )
                } else {
                    (0, 0, 0, 0, 0)
                }
            } else {
                (0, 0, 0, 0, 0)
            };

        // Determine what to log based on benchmark type and extract key size from name
        let is_lookup = self.config.name.contains("lookup");
        let is_disk = self.config.name.contains("disk");
        let key_size_or_count = x_axis_value(&self.config);

        // Mark as failed based on known issue; lookup benchmarks report average
        // per-op latency in milliseconds instead of total elapsed time.
        let status = if self.config.key_len >= 32 && is_lookup {
            "FAILED".to_string()
        } else if is_lookup {
            let avg_latency_ms = (elapsed_secs * 1000.0) / op_cnt as f64;
            format!("{:.6}", avg_latency_ms)
        } else {
            format!("{:.6}", elapsed_secs)
        };
        let io_counts = if is_disk {
            Some(PageIoCounts {
                io_reads,
                io_writes,
                base_page_reads,
                full_page_reads,
                mini_page_reads,
            })
        } else {
            None
        };
        log_benchmark_result(&self.config.name, key_size_or_count, &status, io_counts);

        MicroBenchResult::new(op_cnt, metric).with_elapsed(elapsed_secs)
    }

    fn cleanup(&mut self) -> Option<serde_json::Value> {
        None
    }

    fn on_thread_finished(&mut self, _cur_thread: usize) -> Option<serde_json::Value> {
        let metrics = self.bftree.get_buffer_metrics();
        Some(serde_json::json!({
            "circular_buffer_metrics": metrics,
        }))
    }

    fn load(&mut self) -> Option<serde_json::Value> {
        let mut metrics = bf_tree::metric::TlsRecorder::default();
        let loading_thread = 16;

        // Use preload_records if set, otherwise use record_cnt (for backward compatibility)
        let total_record = if self.config.preload_records > 0 {
            self.config.preload_records
        } else {
            self.config.record_cnt
        };
        let record_per_thread = total_record / loading_thread;
        assert_eq!(total_record % loading_thread, 0);

        let loaded = AtomicUsize::new(0);
        std::thread::scope(|s| {
            let mut handles = vec![];

            for t in 0..loading_thread {
                let tree = &self.bftree;
                let key_len = self.config.key_len;
                let loaded_ref = &loaded;
                let h = s.spawn(move || {
                    bf_tree::metric::get_tls_recorder().reset();
                    let mut buffer = vec![0; key_len / 8];

                    let start = t * record_per_thread;
                    let end = start + record_per_thread;
                    let print_step = record_per_thread / 4;
                    for i in start..end {
                        let key = install_value_to_buffer(&mut buffer, i);
                        tree.insert(key, key);

                        if i % print_step == 0 {
                            loaded_ref.fetch_add(print_step, Ordering::Relaxed);
                            let loaded = loaded_ref.load(Ordering::Relaxed);
                            println!("Loading: {loaded}/{total_record}");
                        }
                    }
                    bf_tree::metric::get_tls_recorder().clone()
                });
                handles.push(h);
            }

            for handle in handles {
                metrics += handle.join().unwrap();
            }
        });

        let metrics = if cfg!(feature = "metrics-rt") {
            Some(metrics)
        } else {
            None
        };

        let circular_buffer_metrics: Option<CircularBufferMetrics> = {
            #[cfg(feature = "metrics-rt")]
            {
                Some(self.bftree.get_buffer_metrics())
            }
            #[cfg(not(feature = "metrics-rt"))]
            {
                None
            }
        };
        Some(serde_json::json!({
            "metrics": metrics,
            "circular_buffer_metrics": circular_buffer_metrics,
        }))
    }
}



pub fn run_bftree_bench(c: BfTreeBench) {
    let mut bench = TestBench::new(&c);
    let results = shumai::run(&mut bench, &c, c.repeat);
    results.write_json().expect("Failed to write json!");

    // Output debug metrics, if any
    let metrics = bench.bftree.get_metrics();
    if let Some(t) = metrics {
        println!("Metrics: {}", t);
    }
}

fn log_benchmark_result(
    config_name: &str,
    value: usize,
    elapsed_secs_or_flag: &str,
    io_counts: Option<PageIoCounts>,
) {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::path::Path;

    let csv_path = std::env::var("BENCHMARK_CSV_PATH")
        .unwrap_or_else(|_| "/tmp/benchmark_results.csv".to_string());
    let file_exists = Path::new(&csv_path).exists();

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&csv_path)
    {
        if !file_exists {
            let is_lookup = config_name.contains("lookup");
            let is_record_count = config_name.contains("record_count");
            let header = if io_counts.is_some() {
                "record_count,avg_latency_ms,io_read_cnt,io_write_cnt,base_page_read_cnt,full_page_read_cnt,mini_page_read_cnt"
            } else if is_lookup && is_record_count {
                "record_count,avg_latency_ms"
            } else if is_lookup {
                "key_size_bytes,avg_latency_ms"
            } else if is_record_count {
                "record_count,elapsed_secs"
            } else {
                "key_size_bytes,elapsed_secs"
            };
            let _ = writeln!(file, "{}", header);
        }
        match io_counts {
            Some(c) => {
                let _ = writeln!(
                    file,
                    "{},{},{},{},{},{},{}",
                    value,
                    elapsed_secs_or_flag,
                    c.io_reads,
                    c.io_writes,
                    c.base_page_reads,
                    c.full_page_reads,
                    c.mini_page_reads
                );
            }
            None => {
                let _ = writeln!(file, "{},{}", value, elapsed_secs_or_flag);
            }
        }
    }
}

// Disk I/O and page-read counters logged for disk-backed lookup benchmarks.
struct PageIoCounts {
    io_reads: usize,
    io_writes: usize,
    base_page_reads: usize,
    full_page_reads: usize,
    mini_page_reads: usize,
}

// Determine the x-axis value to log for a given benchmark config: lookup
// record-count sweeps vary preload_records, insert record-count sweeps vary
// record_cnt, and key-size sweeps (insert or lookup) vary key_len.
fn x_axis_value(config: &BfTreeBench) -> usize {
    let is_lookup = config.name.contains("lookup");
    let is_record_count = config.name.contains("record_count");
    if is_lookup && is_record_count {
        config.preload_records
    } else if is_record_count {
        config.record_cnt
    } else {
        config.key_len
    }
}
