use bitcoincore_rpc::bitcoincore_rpc_json::ListUnspentResultEntry;
use bitcoincore_rpc::{Client, RpcApi};

pub fn fetch_utxos(client: &Client) -> anyhow::Result<Vec<ListUnspentResultEntry>> {
    let utxos = client.list_unspent(None, None, None, None, None)
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("Connection refused") {
                anyhow::anyhow!("Could not connect to Bitcoin Core. Is your node running?\nTip: bitcoind -conf=/home/$USER/.bitcoin/regtest-dev/bitcoin.conf -datadir=/home/$USER/.bitcoin/regtest-dev")
            } else if msg.contains("No wallet") {
                anyhow::anyhow!("No wallet loaded.\nTip: bitcoin-cli -rpcport=18443 -rpcuser=user -rpcpassword=pass loadwallet \"testwallet\"")
            } else {
                anyhow::anyhow!("Failed to fetch UTXOs: {}", msg)
            }
        })?;
    Ok(utxos)
}
