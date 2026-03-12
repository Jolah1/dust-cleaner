use bitcoincore_rpc::bitcoincore_rpc_json::ListUnspentResultEntry;
use bitcoincore_rpc::{Client, RpcApi};

pub struct SweepResult {
    pub psbt: String,
    pub input_count: usize,
    pub total_sats: u64,
}

pub fn build_sweep_psbt(
    client: &Client,
    dust_utxos: &[ListUnspentResultEntry],
) -> anyhow::Result<SweepResult> {
    if dust_utxos.is_empty() {
        anyhow::bail!("No dust UTXOs to sweep");
    }

    // Step 1: Get a fresh address to receive consolidated output
    let change_address = client.get_new_address(None, None)?;
    let change_address = change_address.assume_checked();

    // Step 2: Output — wallet will figure out the right amount after fees
    let outputs = serde_json::json!([{
        change_address.to_string(): "0.0001"
    }]);

    // Step 3: Tell wallet which UTXOs it MUST include (the dust ones)
    let dust_inputs: Vec<serde_json::Value> = dust_utxos
        .iter()
        .map(|utxo| {
            serde_json::json!({
                "txid": utxo.txid.to_string(),
                "vout": utxo.vout,
            })
        })
        .collect();

    // Step 4: Use "inputs" option to mandate dust UTXOs be included
    // wallet will add more inputs automatically if needed to cover fees
    let response = client.call::<serde_json::Value>(
        "walletcreatefundedpsbt",
        &[
            serde_json::json!([]), // let wallet do coin selection
            outputs,
            serde_json::Value::Null, // locktime
            serde_json::json!({
                "subtractFeeFromOutputs": [0],
                "replaceable": true,
                "inputs": dust_inputs  // mandate dust UTXOs are included
            }),
        ],
    )?;

    let psbt = response["psbt"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No PSBT returned from node"))?
        .to_string();

    let total_sats = dust_utxos.iter().map(|u| u.amount.to_sat()).sum();

    Ok(SweepResult {
        psbt,
        input_count: dust_utxos.len(),
        total_sats,
    })
}