use std::{sync::Arc, time::Duration};

use anyhow::Error;
use async_stream::try_stream;
use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive},
        Sse,
    },
    routing::get,
    Router,
};
use futures_util::{Stream, StreamExt};
use log::error;
use serde_json::{from_str, json, Number, Value};
use tokio::{sync::watch, time::interval, try_join};
use tokio_stream::wrappers::IntervalStream;

use crate::state::STATE;

pub struct LatestState {
    latest_blocks_rx: watch::Receiver<Value>,
    latest_txs_rx: watch::Receiver<Value>,
}

pub fn routes() -> Router<()> {
    let (latest_blocks_tx, latest_blocks_rx) = watch::channel(json!(null));
    let (latest_txs_tx, latest_txs_rx) = watch::channel(json!(null));

    let state = Arc::new(LatestState {
        latest_blocks_rx,
        latest_txs_rx,
    });

    tokio::spawn(async move {
        if let Err(e) = async {
            let mut interval = IntervalStream::new(interval(Duration::from_secs(3)));
            while let Some(_) = interval.next().await {
                let (latest_txs, latest_block) = try_join!(get_latest_txs(), get_latest_block())?;
                latest_txs_tx.send_replace(latest_txs);
                latest_blocks_tx.send_replace(latest_block);
            }
            Ok::<(), Error>(())
        }
        .await
        {
            error!("Failed to update latest blocks and txs: {}", e);
            panic!("{:?}", e);
        }
    });

    Router::new()
        .route("/blocks/sse", get(latest_block_sse))
        .route("/txs/sse", get(latest_txs_sse))
        .with_state(state)
}

pub async fn get_latest_block() -> Result<Value, Error> {
    let postgres = STATE.postgres_pool.get().await?;
    let results = postgres
        .query(
            "
            WITH latest_txs AS (
            	SELECT chain_id, block_number FROM transactions ORDER BY id DESC LIMIT 1000
            ),
            txs AS (
            	SELECT *, COUNT(*) AS rtc FROM latest_txs GROUP BY chain_id, block_number
            )
            SELECT blocks.chain_id, number, timestamp, hash, transaction_count, txs.rtc AS related_transaction_count, gas_limit, gas_used FROM blocks LEFT JOIN txs ON blocks.chain_id = txs.chain_id AND blocks.number = txs.block_number WHERE txs.rtc > 0 ORDER BY timestamp DESC, id DESC LIMIT 20
            ",
            &[],
        )
        .await?;
    let datas = results
        .iter()
        .map(|row| {
            Ok(json!({
                "chain_id": row.try_get::<_, i64>("chain_id")?,
                "number": row.try_get::<_, i64>("number")?,
                "timestamp": row.try_get::<_, i64>("timestamp")?,
                "hash": row.try_get::<_, String>("hash")?,
                "transaction_count": row.try_get::<_, i32>("transaction_count")?,
                "related_transaction_count": row.try_get::<_, i64>("related_transaction_count")?,
                "gas_limit": row.try_get::<_, i64>("gas_limit")?,
                "gas_used": row.try_get::<_, i64>("gas_used")?,
            }))
        })
        .collect::<Result<Vec<_>, Error>>()?;

    Ok(json!({
        "data": datas,
    }))
}

pub async fn get_latest_txs() -> Result<Value, Error> {
    let postgres = STATE.postgres_pool.get().await?;
    let limit = 30i64;
    let results = postgres
        .query(
            "
            WITH ltxs AS (SELECT blocks.chain_id, blocks.number as block_number, blocks.timestamp as block_timestamp, transaction_hash, from_address, to_address, value, error, transaction_index, function_signature, ec_pairing_count, ec_recover_addresses FROM transactions INNER JOIN blocks ON blocks.chain_id = transactions.chain_id AND blocks.number = transactions.block_number ORDER BY transactions.id DESC LIMIT $1)
            SELECT ltxs.*, sig_names.name as function_name FROM ltxs LEFT JOIN sig_names ON ltxs.function_signature = sig_names.sig ORDER BY block_timestamp DESC, block_number DESC, transaction_index ASC
            ",
            &[&limit],
        )
        .await?;
    let datas = results
        .iter()
        .map(|row| {
            Ok(json!({
                "chain_id": row.try_get::<_, i64>("chain_id")?,
                "block_number": row.try_get::<_, i64>("block_number")?,
                "block_timestamp": row.try_get::<_, i64>("block_timestamp")?,
                "transaction_hash": row.try_get::<_, String>("transaction_hash")?,
                "transaction_index": row.try_get::<_, i32>("transaction_index")?,
                "from_address": row.try_get::<_, String>("from_address")?,
                "to_address": row.try_get::<_, String>("to_address")?,
                "value": from_str::<Number>(&row.try_get::<_, String>("value")?)?,
                "error": row.try_get::<_, Option<String>>("error")?,
                "ec_pairing_count": row.try_get::<_, i16>("ec_pairing_count")?,
                "ec_recover_addresses": row.try_get::<_, Vec<String>>("ec_recover_addresses")?,
                "function_signature": row.try_get::<_, String>("function_signature")?,
                "function_name": row.try_get::<_, Option<String>>("function_name")?,
            }))
        })
        .collect::<Result<Vec<_>, Error>>()?;

    Ok(json!({
        "data": datas,
    }))
}

pub async fn latest_block_sse(
    State(state): State<Arc<LatestState>>,
) -> Sse<impl Stream<Item = Result<Event, Error>>> {
    let mut rx = state.latest_blocks_rx.clone();
    Sse::new(try_stream! {
        while let Ok(()) = rx.changed().await {
            let data = state.latest_blocks_rx.borrow().clone();
            yield Event::default().json_data(data)?;
        }
    })
    .keep_alive(KeepAlive::default())
}

pub async fn latest_txs_sse(
    State(state): State<Arc<LatestState>>,
) -> Sse<impl Stream<Item = Result<Event, Error>>> {
    let mut rx = state.latest_txs_rx.clone();
    Sse::new(try_stream! {
        while let Ok(()) = rx.changed().await {
            let data = state.latest_txs_rx.borrow().clone();
            yield Event::default().json_data(data)?;
        }
    })
    .keep_alive(KeepAlive::default())
}
