use bitcoincore_rpc::bitcoincore_rpc_json::ListUnspentResultEntry;
use bitcoincore_rpc::{Client, RpcApi};

pub struct SweepResult {
    pub psbt: String,
    pub dust_input_count: usize,
    pub total_dust_sats: u64,
}

pub struct DryRunResult {
    pub dust_input_count: usize,
    pub total_dust_sats: u64,
    pub funder_sats: u64,
    pub estimated_fee_sats: u64,
    pub estimated_output_sats: u64,
}

//helpers

fn select_funder(
    clean_utxos: &[ListUnspentResultEntry],
) -> anyhow::Result<&ListUnspentResultEntry> {
    clean_utxos
        .iter()
        .max_by_key(|u| u.amount.to_sat())
        .ok_or_else(|| anyhow::anyhow!(
            "Cannot sweep: no clean UTXOs available to fund transaction fees.\nTip: fund your wallet first with a non-dust amount."
        ))
}

fn build_inputs(
    funder: &ListUnspentResultEntry,
    dust_utxos: &[ListUnspentResultEntry],
) -> Vec<serde_json::Value> {
    let mut inputs = vec![serde_json::json!({
        "txid": funder.txid.to_string(),
        "vout": funder.vout,
    })];
    for utxo in dust_utxos {
        inputs.push(serde_json::json!({
            "txid": utxo.txid.to_string(),
            "vout": utxo.vout,
        }));
    }
    inputs
}

pub fn build_sweep_psbt(
    client: &Client,
    dust_utxos: &[ListUnspentResultEntry],
    clean_utxos: &[ListUnspentResultEntry],
) -> anyhow::Result<SweepResult> {
    if dust_utxos.is_empty() {
        anyhow::bail!("No dust UTXOs to sweep");
    }

    let funder = select_funder(clean_utxos)?;
    let all_inputs = build_inputs(funder, dust_utxos);

    let out_address = client.get_new_address(None, None)?;
    let out_address = out_address.assume_checked();

    let funder_btc = funder.amount.to_btc();
    let outputs = serde_json::json!([{
        out_address.to_string(): format!("{:.8}", funder_btc)
    }]);

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

pub fn dry_run_sweep(
    dust_utxos: &[ListUnspentResultEntry],
    clean_utxos: &[ListUnspentResultEntry],
) -> anyhow::Result<DryRunResult> {
    if dust_utxos.is_empty() {
        anyhow::bail!("No dust UTXOs to sweep");
    }

    let funder = select_funder(clean_utxos)?;
    let total_dust_sats: u64 = dust_utxos.iter().map(|u| u.amount.to_sat()).sum();
    let funder_sats = funder.amount.to_sat();

    // 68 vbytes per P2WPKH input, 31 vbytes output, 10 vbytes overhead
    // fee rate: 2 sat/vbyte conservative estimate
    let total_inputs = dust_utxos.len() as u64 + 1;
    let estimated_vbytes = (total_inputs * 68) + 31 + 10;
    let estimated_fee_sats = estimated_vbytes * 2;
    let estimated_output_sats = funder_sats + total_dust_sats - estimated_fee_sats;

    Ok(DryRunResult {
        dust_input_count: dust_utxos.len(),
        total_dust_sats,
        funder_sats,
        estimated_fee_sats,
        estimated_output_sats,
    })
}

pub fn build_op_return_psbt(
    client: &Client,
    dust_utxos: &[ListUnspentResultEntry],
    clean_utxos: &[ListUnspentResultEntry],
) -> anyhow::Result<SweepResult> {
    if dust_utxos.is_empty() {
        anyhow::bail!("No dust UTXOs to sweep");
    }

    let funder = select_funder(clean_utxos)?;
    println!(
        "\n   ℹ️  Using clean UTXO to fund fees: {} sats",
        funder.amount.to_sat()
    );

    let all_inputs = build_inputs(funder, dust_utxos);

    // "ash" in hex — ashes to ashes, dust to dust
    let op_return_data = "617368";

    let change_address = client.get_new_address(None, None)?;
    let change_address = change_address.assume_checked();

    let funder_btc = funder.amount.to_btc();
    let outputs = serde_json::json!([
        { "data": op_return_data },
        { change_address.to_string(): format!("{:.8}", funder_btc) }
    ]);

    let response = client.call::<serde_json::Value>(
        "walletcreatefundedpsbt",
        &[
            serde_json::to_value(&all_inputs)?,
            outputs,
            serde_json::Value::Null,
            serde_json::json!({
                "subtractFeeFromOutputs": [1],
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
pub fn build_per_utxo_psbts(
    client: &Client,
    dust_utxos: &[ListUnspentResultEntry],
    clean_utxos: &[ListUnspentResultEntry],
) -> anyhow::Result<Vec<(String, SweepResult)>> {
    if dust_utxos.is_empty() {
        anyhow::bail!("No dust UTXOs to sweep");
    }

    let funder = select_funder(clean_utxos)?;
    let mut results = vec![];

    for utxo in dust_utxos {
        let inputs = vec![
            serde_json::json!({
                "txid": funder.txid.to_string(),
                "vout": funder.vout,
            }),
            serde_json::json!({
                "txid": utxo.txid.to_string(),
                "vout": utxo.vout,
            }),
        ];

        let op_return_data = "617368";
        let change_address = client.get_new_address(None, None)?;
        let change_address = change_address.assume_checked();

        let funder_btc = funder.amount.to_btc();
        let outputs = serde_json::json!([
            { "data": op_return_data },
            { change_address.to_string(): format!("{:.8}", funder_btc) }
        ]);

        let response = client.call::<serde_json::Value>(
            "walletcreatefundedpsbt",
            &[
                serde_json::to_value(&inputs)?,
                outputs,
                serde_json::Value::Null,
                serde_json::json!({
                    "subtractFeeFromOutputs": [1],
                    "replaceable": true,
                }),
            ],
        )?;

        let psbt = response["psbt"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No PSBT returned from node"))?
            .to_string();

        let address = utxo
            .address
            .as_ref()
            .map(|a| a.clone().assume_checked().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        results.push((
            address,
            SweepResult {
                psbt,
                dust_input_count: 1,
                total_dust_sats: utxo.amount.to_sat(),
            },
        ));
    }

    Ok(results)
}
pub struct AnyoneCanPayResult {
    pub address: String,
    pub dust_sats: u64,
    pub raw_tx_hex: String,
}

fn build_anyonecanpay_all_tx(
    client: &Client,
    utxo: &ListUnspentResultEntry,
) -> anyhow::Result<AnyoneCanPayResult> {
    // Step 1: Get the previous transaction scriptPubKey via verbose getrawtransaction
    let prev_tx_info = client.call::<serde_json::Value>(
        "getrawtransaction",
        &[
            serde_json::json!(utxo.txid.to_string()),
            serde_json::json!(true),
        ],
    )?;

    let script_pubkey_hex = prev_tx_info["vout"][utxo.vout as usize]["scriptPubKey"]["hex"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Could not get scriptPubKey"))?;

    let address = utxo
        .address
        .as_ref()
        .map(|a| a.clone().assume_checked().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Step 3: Create the raw transaction hex
    let inputs = serde_json::json!([{
        "txid": utxo.txid.to_string(),
        "vout": utxo.vout,
        "sequence": 4294967293u32
    }]);

    let outputs = serde_json::json!({
        "data": "617368"  // "ash" in hex
    });

    let raw_tx_hex = client.call::<serde_json::Value>(
        "createrawtransaction",
        &[inputs, serde_json::json!([outputs])],
    )?;

    let raw_tx_str = raw_tx_hex
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Could not create raw transaction"))?;

    // Step 4: Sign with SIGHASH_ALL|ANYONECANPAY using wallet
    // sighashtype 0x83 = SIGHASH_ALL | SIGHASH_ANYONECANPAY
    let prevtxs = serde_json::json!([{
        "txid": utxo.txid.to_string(),
        "vout": utxo.vout,
        "scriptPubKey": script_pubkey_hex,
        "amount": utxo.amount.to_btc()
    }]);

    let signed = client.call::<serde_json::Value>(
        "signrawtransactionwithwallet",
        &[
            serde_json::json!(raw_tx_str),
            prevtxs,
            serde_json::json!("ALL|ANYONECANPAY"),
        ],
    )?;

    let complete = signed["complete"].as_bool().unwrap_or(false);

    if !complete {
        let errors = &signed["errors"];
        anyhow::bail!("Signing incomplete: {}", errors);
    }

    let signed_hex = signed["hex"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No signed hex returned"))?
        .to_string();

    Ok(AnyoneCanPayResult {
        address,
        dust_sats: utxo.amount.to_sat(),
        raw_tx_hex: signed_hex,
    })
}

pub fn build_anyonecanpay_all_txs(
    client: &Client,
    dust_utxos: &[ListUnspentResultEntry],
) -> anyhow::Result<Vec<AnyoneCanPayResult>> {
    if dust_utxos.is_empty() {
        anyhow::bail!("No dust UTXOs to sweep");
    }

    let mut results = vec![];
    for utxo in dust_utxos {
        let result = build_anyonecanpay_all_tx(client, utxo)?;
        results.push(result);
    }
    Ok(results)
}
