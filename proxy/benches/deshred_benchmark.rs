use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use jito_shredstream_proxy::deshred::{reconstruct_shreds, ComparableShred, ShredsStateTracker};
use jito_shredstream_proxy::forwarder::ShredMetrics;
use solana_ledger::shred::ReedSolomonCache;
use solana_perf::packet::PacketBatch;
use std::collections::HashSet;
use std::sync::Arc;
use ahash::HashMap;

fn setup_test_state() -> (
    HashMap<u64, (HashMap<u32, HashSet<ComparableShred>>, ShredsStateTracker)>,
    Vec<(u64, u32)>,
    Vec<(u64, Vec<solana_entry::entry::Entry>, Vec<u8>)>,
    u64,
    Arc<ReedSolomonCache>,
    Arc<ShredMetrics>,
) {
    let all_shreds = HashMap::default();
    let slot_fec_indexes = Vec::new();
    let deshredded_entries = Vec::new();
    let highest_slot = 0u64;
    let rs_cache = Arc::new(ReedSolomonCache::default());
    let metrics = Arc::new(ShredMetrics::default());
    
    (all_shreds, slot_fec_indexes, deshredded_entries, highest_slot, rs_cache, metrics)
}

fn load_test_data(path: &str) -> PacketBatch {
    // Try to load test data, fallback to empty batch if file doesn't exist
    match std::fs::read(path) {
        Ok(data) => {
            match bincode::deserialize::<PacketBatch>(&data) {
                Ok(batch) => batch,
                Err(_) => {
                    println!("Warning: Could not deserialize test data from {}, using empty batch", path);
                    PacketBatch::default()
                }
            }
        }
        Err(_) => {
            println!("Warning: Could not load test data from {}, using empty batch", path);
            PacketBatch::default()
        }
    }
}

fn generate_synthetic_data(num_packets: usize) -> PacketBatch {
    use solana_perf::packet::Packet;
    
    let mut batch = PacketBatch::default();
    
    // Generate minimal synthetic packets for benchmarking
    for i in 0..num_packets {
        let mut packet = Packet::default();
        // Fill with minimal data to make it look like a shred
        packet.meta_mut().size = 1024;
        packet.buffer_mut()[0..8].copy_from_slice(&(i as u64).to_le_bytes());
        batch.push(packet);
    }
    
    batch
}

fn benchmark_reconstruct_shreds(c: &mut Criterion) {
    let test_cases = vec![
        ("small_synthetic", generate_synthetic_data(100)),
        ("medium_synthetic", generate_synthetic_data(1000)),
        ("large_synthetic", generate_synthetic_data(5000)),
        ("real_data_small", load_test_data("../bins/serialized_shreds.bin")),
        ("real_data_large", load_test_data("../bins/serialized_shreds_data_complete_test.bin")),
    ];

    let mut group = c.benchmark_group("reconstruct_shreds");
    
    // Set longer measurement time for more stable results
    group.measurement_time(std::time::Duration::from_secs(10));

    for (name, packet_batch) in test_cases {
        if packet_batch.is_empty() && name.contains("real_data") {
            println!("Skipping {} - no test data available", name);
            continue;
        }
        
        group.bench_with_input(BenchmarkId::new("current", name), &packet_batch, |b, packets| {
            b.iter_batched(
                || setup_test_state(),
                |(mut all_shreds, mut slot_fec_indexes, mut deshredded_entries, mut highest_slot, rs_cache, metrics)| {
                    black_box(reconstruct_shreds(
                        packets.clone(),
                        &mut all_shreds,
                        &mut slot_fec_indexes,
                        &mut deshredded_entries,
                        &mut highest_slot,
                        &rs_cache,
                        &metrics,
                    ))
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }
    group.finish();
}

criterion_group!(benches, benchmark_reconstruct_shreds);
criterion_main!(benches);