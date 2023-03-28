use crate::config::Config;
use chainhook_event_observer::bitcoincore_rpc::RpcApi;
use chainhook_event_observer::bitcoincore_rpc::{Auth, Client};
use chainhook_event_observer::chainhooks::bitcoin::{
    evaluate_bitcoin_chainhooks_on_chain_event, handle_bitcoin_hook_action,
    BitcoinChainhookOccurrence, BitcoinTriggerChainhook,
};
use chainhook_event_observer::chainhooks::types::{
    BitcoinChainhookFullSpecification, BitcoinPredicateType, Protocols,
};
use chainhook_event_observer::hord::db::{
    fetch_and_cache_blocks_in_hord_db, find_all_inscriptions, find_compacted_block_at_block_height,
    find_latest_compacted_block_known, open_readonly_hord_db_conn, open_readwrite_hord_db_conn,
};
use chainhook_event_observer::hord::{
    update_storage_and_augment_bitcoin_block_with_inscription_reveal_data,
    update_storage_and_augment_bitcoin_block_with_inscription_transfer_data, Storage,
};
use chainhook_event_observer::indexer;
use chainhook_event_observer::indexer::bitcoin::{
    retrieve_block_hash_with_retry, retrieve_full_block_breakdown_with_retry,
};
use chainhook_event_observer::utils::{file_append, send_request, Context};
use chainhook_types::{BitcoinChainEvent, BitcoinChainUpdatedWithBlocksData};
use std::collections::{BTreeMap, HashMap};

pub async fn scan_bitcoin_chain_with_predicate(
    predicate: BitcoinChainhookFullSpecification,
    config: &Config,
    ctx: &Context,
) -> Result<(), String> {
    let auth = Auth::UserPass(
        config.network.bitcoin_node_rpc_username.clone(),
        config.network.bitcoin_node_rpc_password.clone(),
    );

    let bitcoin_rpc = match Client::new(&config.network.bitcoin_node_rpc_url, auth) {
        Ok(con) => con,
        Err(message) => {
            return Err(format!("Bitcoin RPC error: {}", message.to_string()));
        }
    };

    let predicate_spec =
        match predicate.into_selected_network_specification(&config.network.bitcoin_network) {
            Ok(predicate) => predicate,
            Err(e) => {
                return Err(format!(
                    "Specification missing for network {:?}: {e}",
                    config.network.bitcoin_network
                ));
            }
        };

    let start_block = match predicate_spec.start_block {
        Some(start_block) => start_block,
        None => {
            return Err(
                "Bitcoin chainhook specification must include a field start_block in replay mode"
                    .into(),
            );
        }
    };
    let end_block = match predicate_spec.end_block {
        Some(end_block) => end_block,
        None => match bitcoin_rpc.get_blockchain_info() {
            Ok(result) => result.blocks,
            Err(e) => {
                return Err(format!(
                    "unable to retrieve Bitcoin chain tip ({})",
                    e.to_string()
                ));
            }
        },
    };

    // Are we dealing with an ordinals-based predicate?
    // If so, we could use the ordinal storage to provide a set of hints.
    let mut inscriptions_cache = BTreeMap::new();
    let mut is_predicate_evaluating_ordinals = false;
    let mut hord_blocks_requires_update = false;

    if let BitcoinPredicateType::Protocol(Protocols::Ordinal(_)) = &predicate_spec.predicate {
        is_predicate_evaluating_ordinals = true;
        if let Ok(hord_db_conn) = open_readonly_hord_db_conn(&config.expected_cache_path(), &ctx) {
            inscriptions_cache = find_all_inscriptions(&hord_db_conn);
            // Will we have to update the blocks table?
            if find_compacted_block_at_block_height(end_block as u32, &hord_db_conn).is_none() {
                hord_blocks_requires_update = true;
            }
        }
    }

    // Do we need a seeded hord db?
    if is_predicate_evaluating_ordinals && inscriptions_cache.is_empty() {
        // Do we need to update the blocks table first?
        if hord_blocks_requires_update {
            // Count how many entries in the table
            // Compute the right interval
            // Start the build local storage routine

            // TODO: make sure that we have a contiguous chain
            // check_compacted_blocks_chain_integrity(&hord_db_conn);

            let hord_db_conn = open_readonly_hord_db_conn(&config.expected_cache_path(), ctx)?;

            let start_block = find_latest_compacted_block_known(&hord_db_conn) as u64;
            if start_block < end_block {
                warn!(
                    ctx.expect_logger(),
                    "Database hord.sqlite appears to be outdated regarding the window of blocks provided. Syncing {} missing blocks",
                    (end_block - start_block)
                );
                let rw_hord_db_conn =
                    open_readwrite_hord_db_conn(&config.expected_cache_path(), ctx)?;
                fetch_and_cache_blocks_in_hord_db(
                    &config.get_event_observer_config().get_bitcoin_config(),
                    &rw_hord_db_conn,
                    start_block,
                    end_block,
                    8,
                    &config.expected_cache_path(),
                    &ctx,
                )
                .await?;

                inscriptions_cache = find_all_inscriptions(&hord_db_conn);
            }
        }
    }

    info!(
        ctx.expect_logger(),
        "Starting predicate evaluation on Bitcoin blocks",
    );

    let mut blocks_scanned = 0;
    let mut actions_triggered = 0;

    let event_observer_config = config.get_event_observer_config();
    let bitcoin_config = event_observer_config.get_bitcoin_config();
    let mut traversals = HashMap::new();
    if is_predicate_evaluating_ordinals {
        let hord_db_conn = open_readonly_hord_db_conn(&config.expected_cache_path(), ctx)?;

        let mut storage = Storage::Memory(BTreeMap::new());
        for (cursor, local_traverals) in inscriptions_cache.into_iter() {
            // Only consider inscriptions in the interval specified
            if cursor < start_block || cursor > end_block {
                continue;
            }
            for (transaction_identifier, traversal_result) in local_traverals.into_iter() {
                traversals.insert(transaction_identifier, traversal_result);
            }

            blocks_scanned += 1;

            let block_hash = retrieve_block_hash_with_retry(&cursor, &bitcoin_config, ctx).await?;
            let block_breakdown =
                retrieve_full_block_breakdown_with_retry(&block_hash, &bitcoin_config, ctx).await?;
            let mut block = indexer::bitcoin::standardize_bitcoin_block(
                block_breakdown,
                &event_observer_config.bitcoin_network,
                ctx,
            )?;

            update_storage_and_augment_bitcoin_block_with_inscription_reveal_data(
                &mut block,
                &mut storage,
                &traversals,
                &hord_db_conn,
                &ctx,
            );

            update_storage_and_augment_bitcoin_block_with_inscription_transfer_data(
                &mut block,
                &mut storage,
                &ctx,
            );
            let chain_event =
                BitcoinChainEvent::ChainUpdatedWithBlocks(BitcoinChainUpdatedWithBlocksData {
                    new_blocks: vec![block],
                    confirmed_blocks: vec![],
                });

            let hits = evaluate_bitcoin_chainhooks_on_chain_event(
                &chain_event,
                vec![&predicate_spec],
                ctx,
            );

            actions_triggered += execute_predicates_action(hits, &ctx).await;
        }
    } else {
        let use_scan_to_seed_hord_db = true;

        if use_scan_to_seed_hord_db {
            // Start ingestion pipeline
        }

        for cursor in start_block..=end_block {
            blocks_scanned += 1;
            let block_hash = retrieve_block_hash_with_retry(&cursor, &bitcoin_config, ctx).await?;
            let block_breakdown =
                retrieve_full_block_breakdown_with_retry(&block_hash, &bitcoin_config, ctx).await?;
            let block = indexer::bitcoin::standardize_bitcoin_block(
                block_breakdown,
                &event_observer_config.bitcoin_network,
                ctx,
            )?;

            if use_scan_to_seed_hord_db {
                // Inject new block in ingestion pipeline
                //
                // let _ = cache_block_tx.send(Some((
                //     block_breakdown.height as u32,
                //     CompactedBlock::from_full_block(&block_breakdown),
                // )));
            }

            let chain_event =
                BitcoinChainEvent::ChainUpdatedWithBlocks(BitcoinChainUpdatedWithBlocksData {
                    new_blocks: vec![block],
                    confirmed_blocks: vec![],
                });

            let hits = evaluate_bitcoin_chainhooks_on_chain_event(
                &chain_event,
                vec![&predicate_spec],
                ctx,
            );

            actions_triggered += execute_predicates_action(hits, &ctx).await;
        }
    }
    info!(
        ctx.expect_logger(),
        "{blocks_scanned} blocks scanned, {actions_triggered} actions triggered"
    );

    Ok(())
}

pub async fn execute_predicates_action<'a>(
    hits: Vec<BitcoinTriggerChainhook<'a>>,
    ctx: &Context,
) -> u32 {
    let mut actions_triggered = 0;

    for hit in hits.into_iter() {
        let proofs = HashMap::new();
        match handle_bitcoin_hook_action(hit, &proofs) {
            Err(e) => {
                error!(ctx.expect_logger(), "unable to handle action {}", e);
            }
            Ok(action) => {
                actions_triggered += 1;
                match action {
                    BitcoinChainhookOccurrence::Http(request) => send_request(request, &ctx).await,
                    BitcoinChainhookOccurrence::File(path, bytes) => file_append(path, bytes, &ctx),
                    BitcoinChainhookOccurrence::Data(_payload) => unreachable!(),
                }
            }
        }
    }

    actions_triggered
}
