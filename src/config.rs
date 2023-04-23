use tokio::io::AsyncReadExt;

static CONFIG: tokio::sync::OnceCell<Config> = tokio::sync::OnceCell::const_new();

pub async fn init_config(path: &std::path::Path) -> eyre::Result<&'static Config> {
    let mut cfg_str = String::new();
    tokio::fs::File::open(path)
        .await?
        .read_to_string(&mut cfg_str)
        .await?;
    let config = toml::from_str(&cfg_str)?;
    CONFIG.set(config)?;
    Ok(CONFIG.get().unwrap())
}

pub fn get_config() -> &'static Config {
    CONFIG.get().expect("configs isn't initialized")
}

#[derive(serde::Deserialize, Debug)]
pub struct Config {
    pub discord_token: String,

    pub owners: std::collections::HashSet<u64>,
    #[serde(default = "default_bot_status")]
    pub bot_status: String,
    #[serde(default = "default_bot_activity_type")]
    pub bot_activity_type: String,
    #[serde(default = "default_bot_activity")]
    pub bot_activity: String,
    #[serde(default)]
    pub bot_activity_url: String,
}

fn default_bot_status() -> String {
    "online".to_string()
}

fn default_bot_activity_type() -> String {
    "LISTENING".to_string()
}

fn default_bot_activity() -> String {
    "music".to_string()
}
