use jito_protos::shredstream::{
    shredstream_proxy_client::ShredstreamProxyClient, SubscribeEntriesRequest,
};
use solana_sdk::{message::VersionedMessage, pubkey::Pubkey};
use std::collections::HashSet;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Target accounts we expect to see
    let target_accounts: HashSet<Pubkey> = vec![
        "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA".parse().unwrap(),
        "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P".parse().unwrap(),
    ]
    .into_iter()
    .collect();

    // Additional account to monitor
    let special_account: Pubkey = "39azUYFWPz3VHgKCf3VChUwbpURdCHRxjWVowf5jUJjg".parse().unwrap();

    // let mut client = ShredstreamProxyClient::connect("http://34.159.110.120:9999")
    let mut client = ShredstreamProxyClient::connect("http://34.87.75.45:9999")
        .await
        .unwrap();
    let mut stream = client
        .subscribe_entries(SubscribeEntriesRequest {})
        .await
        .unwrap()
        .into_inner();

    while let Some(slot_entry) = stream.message().await.unwrap() {
        let entries =
            match bincode::deserialize::<Vec<solana_entry::entry::Entry>>(&slot_entry.entries) {
                Ok(e) => e,
                Err(e) => {
                    println!("Deserialization failed with err: {e}");
                    continue;
                }
            };

        let total_transactions = entries.iter().map(|e| e.transactions.len()).sum::<usize>();
        let mut target_account_transactions = 0;
        let mut special_account_transactions = 0;
        let mut all_account_keys: HashSet<Pubkey> = HashSet::new();
        let mut found_target_accounts: HashSet<Pubkey> = HashSet::new();

        // Analyze each transaction
        for entry in &entries {
            for tx in &entry.transactions {
                let account_keys = match &tx.message {
                    VersionedMessage::Legacy(msg) => &msg.account_keys,
                    VersionedMessage::V0(msg) => &msg.account_keys,
                };

                // Check if any account keys match our targets
                let has_target_account = account_keys.iter().any(|k| target_accounts.contains(k));
                if has_target_account {
                    target_account_transactions += 1;
                    // Record which target accounts we found
                    for key in account_keys {
                        if target_accounts.contains(key) {
                            found_target_accounts.insert(*key);
                        }
                    }
                }

                // Check for special account
                let has_special_account = account_keys.iter().any(|k| *k == special_account);
                if has_special_account {
                    special_account_transactions += 1;
                }

                // Collect all unique account keys for analysis
                all_account_keys.extend(account_keys);
            }
        }

        // Only print logs when special account is found
        if special_account_transactions > 0 {
            println!(
                "🎯 SLOT {}: {} entries, {} total txs, {} target txs, {} SPECIAL txs | Found accounts: {:?}",
                slot_entry.slot,
                entries.len(),
                total_transactions,
                target_account_transactions,
                special_account_transactions,
                found_target_accounts.iter().map(|k| k.to_string()[..8].to_string()).collect::<Vec<_>>()
            );

            println!("   🔥 SPECIAL ACCOUNT 39azUYFW found in {} transactions!", special_account_transactions);

            // Show first few account keys for debugging
            if !all_account_keys.is_empty() {
                let sample_keys: Vec<String> = all_account_keys
                    .iter()
                    .take(3)
                    .map(|k| k.to_string()[..8].to_string())
                    .collect();
                println!("   📝 Sample account keys: {:?}...", sample_keys);
            }

            // Alert if no target accounts found (potential filtering issue)
            if target_account_transactions == 0 {
                println!("   ⚠️  WARNING: No transactions with target accounts found!");
            }
        }
    }
    Ok(())
}
