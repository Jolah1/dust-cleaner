use bitcoincore_rpc::{Client, RpcApi};
use bitcoincore_rpc::bitcoincore_rpc_json::ListUnspentResultEntry;

pub fn fetch_utxos(client: &Client) -> anyhow::Result<Vec<ListUnspentResultEntry>> {
    let utxos = client.list_unspent(None, None, None, None, None)?;
    Ok(utxos)
}