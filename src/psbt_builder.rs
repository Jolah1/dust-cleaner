use bitcoincore_rpc::bitcoincore_rpc_json::ListUnspentResultEntry;
use bitcoincore_rpc::{Client, RpcApi};

pub struct SweepResult {
    pub psbt: String,
    pub dust_input_count: usize,
    pub total_dust_sats: u64,
}

pub fn build_sweep_psbt(
    client: &Client,
    dust_utxos: &[ListUnspentResultEntry],
    clean_utxos: &[ListUnspentResultEntry],
) -> anyhow::Result<SweepResult> {
    if dust_utxos.is_empty() {
        anyhow::bail!("No dust UTXOs to sweep");
    }

    // Step 1: Pick the largest clean UTXO to fund the transaction
    let funder = clean_utxos
        .iter()
        .max_by_key(|u| u.amount.to_sat())
        .ok_or_else(|| anyhow::anyhow!("No clean UTXOs available to fund fee"))?;

    println!("\n   ℹ️  Using clean UTXO to fund fees: {} sats", funder.amount.to_sat());

    // Step 2: Build inputs — funder first, then all dust UTXOs
    let mut all_inputs: Vec<serde_json::Value> = vec![
        serde_json::json!({
            "txid": funder.txid.to_string(),
            "vout": funder.vout,
        })
    ];

    for utxo in dust_utxos {
        all_inputs.push(serde_json::json!({
            "txid": utxo.txid.to_string(),
            "vout": utxo.vout,
        }));
    }

    // Step 3: Get a fresh address for consolidated output
    let out_address = client.get_new_address(None, None)?;
    let out_address = out_address.assume_checked();

    // Step 4: Output amount = funder amount (fees subtracted automatically)
    // We use the funder's full amount and subtract fees from it
    let funder_btc = funder.amount.to_btc();
    let outputs = serde_json::json!([{
        out_address.to_string(): format!("{:.8}", funder_btc)
    }]);

    // Step 5: Create PSBT — subtract fee from output, wallet handles the rest
    let response = client.call::<serde_json::Value>(
        "walletcreatefundedpsbt",
        &[
            serde_json::to_value(&all_inputs)?,
            outputs,
            serde_json::Value::Null,
            serde_json::json!({
                "subtractFeeFromOutputs": [0],
                "replaceable": true,
            }),
        ],
    )?;

    let psbt = response["psbt"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No PSBT returned from node"))?
        .to_string();

    let total_dust_sats = dust_utxos.iter().map(|u| u.amount.to_sat()).sum();

    Ok(SweepResult {
        psbt,
        dust_input_count: dust_utxos.len(),
        total_dust_sats,
    })
}