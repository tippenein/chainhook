use std::collections::HashMap;

use self::fixtures::get_all_event_types;

use super::{
    stacks::{evaluate_stacks_chainhooks_on_chain_event, StacksTriggerChainhook, handle_stacks_hook_action, StacksChainhookOccurrence},
    types::{StacksChainhookSpecification, StacksPrintEventBasedPredicate, StacksNftEventBasedPredicate, StacksFtEventBasedPredicate,StacksContractCallBasedPredicate,StacksContractDeploymentPredicate, ExactMatchingRule, FileHook, StacksTrait},
};
use crate::{chainhooks::{types::{HookAction, StacksPredicate, StacksStxEventBasedPredicate,}, tests::fixtures::{get_expected_occurrence, get_test_event_by_type}}, utils::AbstractStacksBlock};
use crate::utils::Context;
use chainhook_types::{StacksNetwork, StacksTransactionEvent, StacksTransactionData};
use chainhook_types::{StacksBlockUpdate, StacksChainEvent, StacksChainUpdatedWithBlocksData};
use test_case::test_case;
use serde_json::Value as JsonValue;

pub mod fixtures;

// FtEvent predicate tests
#[test_case(
    vec![vec![get_test_event_by_type("ft_mint")]], 
    StacksPredicate::FtEvent(StacksFtEventBasedPredicate {
        asset_identifier: "asset-id".to_string(),
        actions: vec!["mint".to_string()]
    }),
    1;
    "FtEvent predicates match mint event"
)]
#[test_case(
    vec![vec![get_test_event_by_type("ft_transfer")]], 
    StacksPredicate::FtEvent(StacksFtEventBasedPredicate {
        asset_identifier: "asset-id".to_string(),
        actions: vec!["transfer".to_string()]
    }),
    1;
    "FtEvent predicates match transfer event"
)]
#[test_case(
    vec![vec![get_test_event_by_type("ft_burn")]], 
    StacksPredicate::FtEvent(StacksFtEventBasedPredicate {
        asset_identifier: "asset-id".to_string(),
        actions: vec!["burn".to_string()]
    }),
    1;
    "FtEvent predicates match burn event"
)]
#[test_case(
    vec![vec![get_test_event_by_type("ft_mint")]], 
    StacksPredicate::FtEvent(StacksFtEventBasedPredicate {
        asset_identifier: "wrong-id".to_string(),
        actions: vec!["mint".to_string()]
    }),
    0 => ignore;
    "FtEvent predicates reject no-match asset id for mint event"
)]
#[test_case(
    vec![vec![get_test_event_by_type("ft_transfer")]], 
    StacksPredicate::FtEvent(StacksFtEventBasedPredicate {
        asset_identifier: "wrong-id".to_string(),
        actions: vec!["transfer".to_string()]
    }),
    0 => ignore;
    "FtEvent predicates reject no-match asset id for transfer event"
)]
#[test_case(
    vec![vec![get_test_event_by_type("ft_burn")]], 
    StacksPredicate::FtEvent(StacksFtEventBasedPredicate {
        asset_identifier: "wrong-id".to_string(),
        actions: vec!["burn".to_string()]
    }),
    0 => ignore;
    "FtEvent predicates reject no-match asset id for burn event"
)]
#[test_case(
    vec![vec![get_test_event_by_type("ft_mint")],vec![get_test_event_by_type("ft_transfer")],vec![get_test_event_by_type("ft_burn")]], 
    StacksPredicate::FtEvent(StacksFtEventBasedPredicate {
        asset_identifier: "asset-id".to_string(),
        actions: vec!["mint".to_string(),"transfer".to_string(), "burn".to_string()]
    }),
    3;
    "FtEvent predicates match multiple events"
)]
#[test_case(
    vec![vec![get_test_event_by_type("ft_transfer")],vec![get_test_event_by_type("ft_burn")]], 
    StacksPredicate::FtEvent(StacksFtEventBasedPredicate {
        asset_identifier: "asset-id".to_string(),
        actions: vec!["mint".to_string()]
    }),
    0;
    "FtEvent predicates don't match if missing event"
)]

// NftEvent predicate tests
#[test_case(
    vec![vec![get_test_event_by_type("nft_mint")]], 
    StacksPredicate::NftEvent(StacksNftEventBasedPredicate {
        asset_identifier: "asset-id".to_string(),
        actions: vec!["mint".to_string()]
    }),
    1;
    "NftEvent predicates match mint event"
)]
#[test_case(
    vec![vec![get_test_event_by_type("nft_transfer")    ]], 
    StacksPredicate::NftEvent(StacksNftEventBasedPredicate {
        asset_identifier: "asset-id".to_string(),
        actions: vec!["transfer".to_string()]
    }),
    1;
    "NftEvent predicates match transfer event"
)]
#[test_case(
    vec![vec![get_test_event_by_type("nft_burn")]], 
    StacksPredicate::NftEvent(StacksNftEventBasedPredicate {
        asset_identifier: "asset-id".to_string(),
        actions: vec!["burn".to_string()]
    }),
    1;
    "NftEvent predicates match burn event"
)]
#[test_case(
    vec![vec![get_test_event_by_type("nft_mint")],vec![get_test_event_by_type("nft_transfer")],vec![get_test_event_by_type("nft_burn")]], 
    StacksPredicate::NftEvent(StacksNftEventBasedPredicate {
        asset_identifier: "asset-id".to_string(),
        actions: vec!["mint".to_string(),"transfer".to_string(), "burn".to_string()]
    }),
    3;
    "NftEvent predicates match multiple events"
)]
#[test_case(
    vec![vec![get_test_event_by_type("nft_transfer")],vec![get_test_event_by_type("nft_burn")]], 
    StacksPredicate::NftEvent(StacksNftEventBasedPredicate {
        asset_identifier: "asset-id".to_string(),
        actions: vec!["mint".to_string()]
    }),
    0;
    "NftEvent predicates don't match if missing event"
)]
// StxEvent predicate tests
#[test_case(
    vec![vec![get_test_event_by_type("stx_mint")]], 
    StacksPredicate::StxEvent(StacksStxEventBasedPredicate {
        actions: vec!["mint".to_string()]
    }),
    1;
    "StxEvent predicates match mint event"
)]
#[test_case(
    vec![vec![get_test_event_by_type("stx_transfer")]], 
    StacksPredicate::StxEvent(StacksStxEventBasedPredicate {
        actions: vec!["transfer".to_string()]
    }),
    1;
    "StxEvent predicates match transfer event"
)]
#[test_case(
    vec![vec![get_test_event_by_type("stx_lock")]], 
    StacksPredicate::StxEvent(StacksStxEventBasedPredicate {
        actions: vec!["lock".to_string()]
    }),
    1;
    "StxEvent predicates match lock event"
)]
#[test_case(
    vec![vec![get_test_event_by_type("stx_burn")]], 
    StacksPredicate::StxEvent(StacksStxEventBasedPredicate {
        actions: vec!["burn".to_string()]
    }),
    1 => ignore;
    "StxEvent predicates match burn event"
)]
#[test_case(
    vec![vec![get_test_event_by_type("stx_mint")],vec![get_test_event_by_type("stx_transfer")],vec![get_test_event_by_type("stx_lock")]], 
    StacksPredicate::StxEvent(StacksStxEventBasedPredicate {
        actions: vec!["mint".to_string(), "transfer".to_string(), "lock".to_string()]
    }),
    3;
    "StxEvent predicates match multiple events"
)]
#[test_case(
    vec![vec![get_test_event_by_type("stx_transfer")],vec![get_test_event_by_type("stx_lock")]], 
    StacksPredicate::StxEvent(StacksStxEventBasedPredicate {
        actions: vec!["mint".to_string()]
    }),
    0;
    "StxEvent predicates don't match if missing event"
)]

// PrintEvent predicate tests
#[test_case(
    vec![vec![get_test_event_by_type("smart_contract_print_event")]], 
    StacksPredicate::PrintEvent(StacksPrintEventBasedPredicate {
        contract_identifier: "ST3AXH4EBHD63FCFPTZ8GR29TNTVWDYPGY0KDY5E5.loan-data".to_string(),
        contains: "some-value".to_string()
    }),
    1;
    "PrintEvent predicate matches contract_identifier and contains"
)]
#[test_case(
    vec![vec![get_test_event_by_type("smart_contract_not_print_event")]], 
    StacksPredicate::PrintEvent(StacksPrintEventBasedPredicate {
        contract_identifier: "ST3AXH4EBHD63FCFPTZ8GR29TNTVWDYPGY0KDY5E5.loan-data".to_string(),
        contains: "some-value".to_string(),
    }),
    0;
    "PrintEvent predicate does not check events with topic other than print"
)]
#[test_case(
    vec![vec![get_test_event_by_type("smart_contract_print_event")]], 
    StacksPredicate::PrintEvent(StacksPrintEventBasedPredicate {
        contract_identifier: "wront-id".to_string(),
        contains: "some-value".to_string(),
    }), 
    0;
    "PrintEvent predicate rejects non matching contract_identifier"
)]
#[test_case(
    vec![vec![get_test_event_by_type("smart_contract_print_event")]], 
    StacksPredicate::PrintEvent(StacksPrintEventBasedPredicate {
        contract_identifier: 
            "ST3AXH4EBHD63FCFPTZ8GR29TNTVWDYPGY0KDY5E5.loan-data".to_string(),
        contains: "wrong-value".to_string(),
    }), 
    0;
    "PrintEvent predicate rejects non matching contains value"
)]
#[test_case(
    vec![vec![get_test_event_by_type("smart_contract_print_event")]], 
    StacksPredicate::PrintEvent(StacksPrintEventBasedPredicate {
        contract_identifier: "*".to_string(),
        contains: "some-value".to_string(),
    }), 
    1;
    "PrintEvent predicate contract_identifier wildcard checks all print events for match"
)]
#[test_case(
    vec![vec![get_test_event_by_type("smart_contract_print_event")]], 
    StacksPredicate::PrintEvent(StacksPrintEventBasedPredicate {
        contract_identifier: "ST3AXH4EBHD63FCFPTZ8GR29TNTVWDYPGY0KDY5E5.loan-data".to_string(),
        contains: "*".to_string(),
    }), 
    1;
    "PrintEvent predicate contains wildcard matches all values for matching events"
)]
#[test_case(
    vec![vec![get_test_event_by_type("smart_contract_print_event")], vec![get_test_event_by_type("smart_contract_print_event_empty")]], 
    StacksPredicate::PrintEvent(StacksPrintEventBasedPredicate {
        contract_identifier: "*".to_string(),
        contains: "*".to_string(),
    }), 
    2;
    "PrintEvent predicate contract_identifier wildcard and contains wildcard matches all values on all print events"
)]
fn test_stacks_predicates(blocks_with_events: Vec<Vec<StacksTransactionEvent>>, predicate: StacksPredicate, expected_applies: u64) {
    // Prepare block
    let new_blocks = blocks_with_events.iter().map(|events| StacksBlockUpdate {
        block: fixtures::build_stacks_testnet_block_from_smart_contract_event_data(events),
        parent_microblocks_to_apply: vec![],
        parent_microblocks_to_rollback: vec![],
    }).collect();
    let event = StacksChainEvent::ChainUpdatedWithBlocks(StacksChainUpdatedWithBlocksData {
        new_blocks,
        confirmed_blocks: vec![],
    });
    // Prepare predicate
    let chainhook = StacksChainhookSpecification {
        uuid: "".to_string(),
        owner_uuid: None,
        name: "".to_string(),
        network: StacksNetwork::Testnet,
        version: 1,
        blocks: None,
        start_block: None,
        end_block: None,
        expire_after_occurrence: None,
        capture_all_events: None,
        decode_clarity_values: None,
        predicate: predicate,
        action: HookAction::Noop,
        enabled: true,
    };

    let predicates = vec![&chainhook];
    let (triggered, _blocks) =
        evaluate_stacks_chainhooks_on_chain_event(&event, predicates, &Context::empty());

    if expected_applies == 0 {
        assert_eq!(triggered.len(), 0)
    }
    else {
        let actual_applies: u64 = triggered[0].apply.len().try_into().unwrap();
        assert_eq!(actual_applies, expected_applies);
    }
}


#[test_case(
    StacksPredicate::ContractDeployment(StacksContractDeploymentPredicate::Deployer("ST13F481SBR0R7Z6NMMH8YV2FJJYXA5JPA0AD3HP9".to_string())), 
    1;
    "Deployer predicate matches by contract deployer"
)]
#[test_case(
    StacksPredicate::ContractDeployment(StacksContractDeploymentPredicate::Deployer("*".to_string())), 
    1;
    "Deployer predicate wildcard deployer catches all occurences"
)]
#[test_case(
    StacksPredicate::ContractDeployment(StacksContractDeploymentPredicate::Deployer("wrong-deployer".to_string())), 
    0;
    "Deployer predicate does not match non-matching deployer"
)]
#[test_case(
    StacksPredicate::ContractDeployment(StacksContractDeploymentPredicate::ImplementTrait(StacksTrait::Sip09)), 
    0;
    "ImplementSip predicate returns no values for Sip09"
)]
#[test_case(
    StacksPredicate::ContractDeployment(StacksContractDeploymentPredicate::ImplementTrait(StacksTrait::Sip10)), 
    0;
    "ImplementSip predicate returns no values for Sip10"
)]
#[test_case(
    StacksPredicate::ContractDeployment(StacksContractDeploymentPredicate::ImplementTrait(StacksTrait::Any)), 
    0;
    "ImplementSip predicate returns no values for Any"
)]
fn test_stacks_predicate_contract_deploy(predicate: StacksPredicate, expected_applies: u64) {
    // Prepare block
    let new_blocks = vec![StacksBlockUpdate {
        block: fixtures::build_stacks_testnet_block_with_contract_deployment(),
        parent_microblocks_to_apply: vec![],
        parent_microblocks_to_rollback: vec![],
    }, StacksBlockUpdate {
        block: fixtures::build_stacks_testnet_block_with_contract_call(),
        parent_microblocks_to_apply: vec![],
        parent_microblocks_to_rollback: vec![],
    }];
    let event = StacksChainEvent::ChainUpdatedWithBlocks(StacksChainUpdatedWithBlocksData {
        new_blocks,
        confirmed_blocks: vec![],
    });
    // Prepare predicate
    let chainhook = StacksChainhookSpecification {
        uuid: "".to_string(),
        owner_uuid: None,
        name: "".to_string(),
        network: StacksNetwork::Testnet,
        version: 1,
        blocks: None,
        start_block: None,
        end_block: None,
        expire_after_occurrence: None,
        capture_all_events: None,
        decode_clarity_values: None,
        predicate: predicate,
        action: HookAction::Noop,
        enabled: true,
    };

    let predicates = vec![&chainhook];
    let (triggered, _blocks) =
        evaluate_stacks_chainhooks_on_chain_event(&event, predicates, &Context::empty());

    if expected_applies == 0 {
        assert_eq!(triggered.len(), 0)
    }
    else if triggered.len() == 0 {
        panic!("expected more than one block to be applied, but no predicates were triggered")
    }
    else {
        let actual_applies: u64 = triggered[0].apply.len().try_into().unwrap();
        assert_eq!(actual_applies, expected_applies);
    }
}

#[test_case(
    StacksPredicate::ContractCall(StacksContractCallBasedPredicate {
        contract_identifier: "ST13F481SBR0R7Z6NMMH8YV2FJJYXA5JPA0AD3HP9.subnet-v1".to_string(),
        method: "commit-block".to_string()
    }), 
    1;
    "ContractCall predicate matches by contract identifier and method"
)]
#[test_case(
    StacksPredicate::ContractCall(StacksContractCallBasedPredicate {
        contract_identifier: "ST13F481SBR0R7Z6NMMH8YV2FJJYXA5JPA0AD3HP9.subnet-v1".to_string(),
        method: "wrong-method".to_string()
    }), 
    0;
    "ContractCall predicate does not match for wrong method"
)]
#[test_case(
    StacksPredicate::ContractCall(StacksContractCallBasedPredicate {
        contract_identifier: "wrong-id".to_string(),
        method: "commit-block".to_string()
    }), 
    0;
    "ContractCall predicate does not match for wrong contract identifier"
)]
#[test_case(
    StacksPredicate::Txid(ExactMatchingRule::Equals("0xb92c2ade84a8b85f4c72170680ae42e65438aea4db72ba4b2d6a6960f4141ce8".to_string())), 
    1;
    "Txid predicate matches by a transaction's id"
)]
#[test_case(
    StacksPredicate::Txid(ExactMatchingRule::Equals("wrong-id".to_string())), 
    0;
    "Txid predicate rejects non matching id"
)]
fn test_stacks_predicate_contract_call(predicate: StacksPredicate, expected_applies: u64) {
    // Prepare block
    let new_blocks = vec![StacksBlockUpdate {
        block: fixtures::build_stacks_testnet_block_with_contract_call(),
        parent_microblocks_to_apply: vec![],
        parent_microblocks_to_rollback: vec![],
    },StacksBlockUpdate {
        block: fixtures::build_stacks_testnet_block_with_contract_deployment(),
        parent_microblocks_to_apply: vec![],
        parent_microblocks_to_rollback: vec![],
    }];
    let event = StacksChainEvent::ChainUpdatedWithBlocks(StacksChainUpdatedWithBlocksData {
        new_blocks,
        confirmed_blocks: vec![],
    });
    // Prepare predicate
    let chainhook = StacksChainhookSpecification {
        uuid: "".to_string(),
        owner_uuid: None,
        name: "".to_string(),
        network: StacksNetwork::Testnet,
        version: 1,
        blocks: None,
        start_block: None,
        end_block: None,
        expire_after_occurrence: None,
        capture_all_events: None,
        decode_clarity_values: None,
        predicate: predicate,
        action: HookAction::Noop,
        enabled: true,
    };

    let predicates = vec![&chainhook];
    let (triggered, _blocks) =
        evaluate_stacks_chainhooks_on_chain_event(&event, predicates, &Context::empty());

    if expected_applies == 0 {
        assert_eq!(triggered.len(), 0)
    }
    else if triggered.len() == 0 {
        panic!("expected more than one block to be applied, but no predicates were triggered")
    }
    else {
        let actual_applies: u64 = triggered[0].apply.len().try_into().unwrap();
        assert_eq!(actual_applies, expected_applies);
    }
}

#[test]
fn test_stacks_hook_action_noop() {
    let chainhook = StacksChainhookSpecification {
        uuid: "".to_string(),
        owner_uuid: None,
        name: "".to_string(),
        network: StacksNetwork::Testnet,
        version: 1,
        blocks: None,
        start_block: None,
        end_block: None,
        expire_after_occurrence: None,
        capture_all_events: None,
        decode_clarity_values: None,
        predicate: StacksPredicate::Txid(ExactMatchingRule::Equals("0xb92c2ade84a8b85f4c72170680ae42e65438aea4db72ba4b2d6a6960f4141ce8".to_string())),
        action: HookAction::Noop,
        enabled: true,
    };


    let apply_block_data = fixtures::build_stacks_testnet_block_with_contract_call();
    let apply_transactions = apply_block_data.transactions.iter().map(|t|t).collect();
    let apply_blocks: &dyn AbstractStacksBlock = &apply_block_data;

    let rollback_block_data = fixtures::build_stacks_testnet_block_with_contract_deployment();
    let rollback_transactions = rollback_block_data.transactions.iter().map(|t|t).collect();
    let rollback_blocks: &dyn AbstractStacksBlock = &apply_block_data;
    let trigger = StacksTriggerChainhook {
        chainhook: &chainhook,
        apply: vec![(apply_transactions, apply_blocks)],
        rollback: vec![(rollback_transactions, rollback_blocks)]
    };

    let proofs = HashMap::new();
    let ctx = Context { logger: None, tracer: false };
    let occurrence = handle_stacks_hook_action(trigger, &proofs, &ctx).unwrap();
    if let StacksChainhookOccurrence::Data(data) = occurrence {
        assert_eq!(data.apply.len(), 1);
        assert_eq!(data.apply[0].block_identifier.hash, apply_block_data.block_identifier.hash);
        assert_eq!(data.rollback.len(), 1);
        assert_eq!(data.rollback[0].block_identifier.hash, rollback_block_data.block_identifier.hash);
    }
    else {
        panic!("wrong occurrence type");
    }
}


#[test]
fn test_stacks_hook_action_file_append() {
    let chainhook = StacksChainhookSpecification {
        uuid: "".to_string(),
        owner_uuid: None,
        name: "".to_string(),
        network: StacksNetwork::Testnet,
        version: 1,
        blocks: None,
        start_block: None,
        end_block: None,
        expire_after_occurrence: None,
        capture_all_events: None,
        decode_clarity_values: Some(true),
        predicate: StacksPredicate::Txid(ExactMatchingRule::Equals("0xb92c2ade84a8b85f4c72170680ae42e65438aea4db72ba4b2d6a6960f4141ce8".to_string())),
        action: HookAction::FileAppend(FileHook {path: "./".to_string()}),
        enabled: true,
    };
    let events = get_all_event_types();
    let mut apply_blocks = vec![];
    for event in events.iter() {
        apply_blocks.push(fixtures::build_stacks_testnet_block_from_smart_contract_event_data(&vec![event.to_owned()]));

    }
    let apply: Vec<(Vec<&StacksTransactionData>, &dyn AbstractStacksBlock)> = apply_blocks.iter().map(|b| (b.transactions.iter().map(|t| t).collect(), b as &dyn AbstractStacksBlock)).collect();


    let rollback_block_data = fixtures::build_stacks_testnet_block_with_contract_deployment();
    let rollback_transactions = rollback_block_data.transactions.iter().map(|t|t).collect();
    let rollback_block: &dyn AbstractStacksBlock = &rollback_block_data;
    let trigger = StacksTriggerChainhook {
        chainhook: &chainhook,
        apply: apply,
        rollback: vec![(rollback_transactions, rollback_block)]
    };

    let proofs = HashMap::new();
    let ctx = Context { logger: None, tracer: false };
    let occurrence = handle_stacks_hook_action(trigger, &proofs, &ctx).unwrap();
    if let StacksChainhookOccurrence::File(path, bytes) = occurrence {
        assert_eq!(path, "./".to_string());
        let json: JsonValue = serde_json::from_slice(&bytes).unwrap();
        let obj = json.as_object().unwrap();
        let actual = serde_json::to_string_pretty(obj).unwrap();
        let expected = get_expected_occurrence();
        assert_eq!(expected, actual);
    }
    else {
        panic!("wrong occurence type");
    }
}