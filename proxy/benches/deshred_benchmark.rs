use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use jito_shredstream_proxy::deshred::{reconstruct_shreds, ComparableShred, ShredsStateTracker};
use jito_shredstream_proxy::forwarder::ShredMetrics;
use solana_ledger::shred::{ReedSolomonCache, merkle::Shred};
use solana_perf::packet::{PacketBatch, Packet};
use std::collections::HashSet;
use std::sync::Arc;
use ahash::HashMap;
use borsh::BorshDeserialize;

#[derive(BorshDeserialize)]
struct Packets {
    pub packets: Vec<Vec<u8>>,
}

fn setup_test_state() -> (
    HashMap<u64, (HashMap<u32, HashSet<ComparableShred>>, ShredsStateTracker)>,
    Vec<(u64, u32)>,
    Vec<(u64, Vec<solana_entry::entry::Entry>, Vec<u8>)>,
    u64,
    Arc<ReedSolomonCache>,
    Arc<ShredMetrics>,
    Vec<Shred>,
) {
    let all_shreds = HashMap::default();
    let slot_fec_indexes = Vec::new();
    let deshredded_entries = Vec::new();
    let highest_slot = 0u64;
    let rs_cache = Arc::new(ReedSolomonCache::default());
    let metrics = Arc::new(ShredMetrics::default());
    let merkle_shreds_buffer = Vec::with_capacity(64);
    
    (all_shreds, slot_fec_indexes, deshredded_entries, highest_slot, rs_cache, metrics, merkle_shreds_buffer)
}

fn load_test_data(path: &str) -> PacketBatch {
    // Try to load test data, fallback to empty batch if file doesn't exist
    match std::fs::read(path) {
        Ok(data) => {
            match Packets::try_from_slice(&data) {
                Ok(packets) => {
                    // Convert Vec<Vec<u8>> to PacketBatch
                    let mut batch = PacketBatch::default();
                    for packet_data in packets.packets.into_iter().take(500) { // Limit for benchmarking
                        let mut packet = Packet::default();
                        let len = std::cmp::min(packet_data.len(), 1500); // Max packet size
                        packet.buffer_mut()[..len].copy_from_slice(&packet_data[..len]);
                        packet.meta_mut().size = len;
                        batch.push(packet);
                    }
                    batch
                }
                Err(e) => {
                    println!("Warning: Could not deserialize test data from {}: {}, using empty batch", path, e);
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
    
    // Configure for different test data types
    group.measurement_time(std::time::Duration::from_secs(15));
    group.sample_size(50); // Reduce sample count for slower benchmarks

    for (name, packet_batch) in test_cases {
        if packet_batch.is_empty() && name.contains("real_data") {
            println!("Skipping {} - no test data available", name);
            continue;
        }
        
        group.bench_with_input(BenchmarkId::new("current", name), &packet_batch, |b, packets| {
            b.iter_batched(
                || setup_test_state(),
                |(mut all_shreds, mut slot_fec_indexes, mut deshredded_entries, mut highest_slot, rs_cache, metrics, mut merkle_shreds_buffer)| {
                    black_box(reconstruct_shreds(
                        packets.clone(),
                        &mut all_shreds,
                        &mut slot_fec_indexes,
                        &mut deshredded_entries,
                        &mut highest_slot,
                        &rs_cache,
                        &metrics,
                        &mut merkle_shreds_buffer,
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