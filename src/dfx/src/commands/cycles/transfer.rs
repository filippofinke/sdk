use crate::lib::environment::Environment;
use crate::lib::error::DfxResult;
use crate::lib::nns_types::account_identifier::Subaccount;
use crate::lib::operations::cycles_ledger;
use crate::lib::root_key::fetch_root_key_if_needed;
use crate::util::clap::parsers::cycle_amount_parser;
use candid::Principal;
use clap::Parser;
use icrc_ledger_types::icrc1;
use slog::warn;
use std::time::{SystemTime, UNIX_EPOCH};

/// Transfer cycles to another principal.
#[derive(Parser)]
pub struct TransferOpts {
    /// Transfer cycles to this principal.
    to: Principal,

    /// The number of cycles to send.
    #[arg(value_parser = cycle_amount_parser)]
    amount: u128,

    /// Transfer cycles from this principal. Requires that principal's approval.
    #[arg(long)]
    from: Option<Principal>,

    /// Transfer cycles from this subaccount.
    #[arg(long)]
    from_subaccount: Option<Subaccount>,

    /// Deduct allowance from this subaccount.
    #[arg(long, requires("from"))]
    spender_subaccount: Option<Subaccount>,

    /// Transfer cycles to this subaccount.
    #[arg(long)]
    to_subaccount: Option<Subaccount>,

    /// Transaction timestamp, in nanoseconds, for use in controlling transaction-deduplication, default is system-time.
    /// https://internetcomputer.org/docs/current/developer-docs/integrations/icrc-1/#transaction-deduplication-
    #[arg(long)]
    created_at_time: Option<u64>,

    /// Memo.
    #[arg(long)]
    memo: Option<u64>,
}

pub async fn exec(env: &dyn Environment, opts: TransferOpts) -> DfxResult {
    let agent = env.get_agent();

    let amount = opts.amount;

    fetch_root_key_if_needed(env).await?;

    let created_at_time = opts.created_at_time.unwrap_or(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64,
    );

    let from_subaccount = opts.from_subaccount.map(|x| x.0);
    let to_subaccount = opts.to_subaccount.map(|x| x.0);

    let result = if let Some(from_owner) = opts.from {
        let spender_subaccount = opts.spender_subaccount.map(|x| x.0);
        let from = icrc1::account::Account {
            owner: from_owner,
            subaccount: from_subaccount,
        };
        let to = icrc1::account::Account {
            owner: opts.to,
            subaccount: to_subaccount,
        };
        cycles_ledger::transfer_from(
            agent,
            env.get_logger(),
            spender_subaccount,
            from,
            to,
            amount,
            opts.memo,
            created_at_time,
        )
        .await
    } else {
        cycles_ledger::transfer(
            agent,
            env.get_logger(),
            amount,
            from_subaccount,
            opts.to,
            to_subaccount,
            created_at_time,
            opts.memo,
        )
        .await
    };

    if result.is_err() && opts.created_at_time.is_none() {
        warn!(
            env.get_logger(),
            "If you retry this operation, use --created-at-time {}", created_at_time
        );
    }
    let block_index = result?;

    println!("Transfer sent at block index {block_index}");

    Ok(())
}
