use bitcoincore_rpc::{Auth, Client};

pub fn connect(url: &str, user: &str, pass: &str) -> anyhow::Result<Client> {
    let client = Client::new(
        url,
        Auth::UserPass(user.into(), pass.into()),
    ).map_err(|e| anyhow::anyhow!(
        "Failed to create RPC client: {}\nCheck your --rpc-url, --rpc-user and --rpc-pass", e
    ))?;
    Ok(client)
}