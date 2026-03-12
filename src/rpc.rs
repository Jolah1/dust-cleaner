use bitcoincore_rpc::{Auth, Client};

pub fn connect(url: &str, user: &str, pass: &str) -> anyhow::Result<Client> {
    let client = Client::new(
        url,
        Auth::UserPass(user.into(), pass.into()),
    )?;
    Ok(client)
}