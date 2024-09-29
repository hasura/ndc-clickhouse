use common::{client::get_http_client, config::ServerConfig};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct ServerState {
    client: Arc<RwLock<Option<reqwest::Client>>>,
}

impl ServerState {
    pub fn new(config: &ServerConfig) -> ServerState {
        // if client creation fails for whatever reason, client should be none.
        let client = get_http_client(&config.connection).ok();

        ServerState {
            client: Arc::new(RwLock::new(client)),
        }
    }
    pub async fn client(&self, config: &ServerConfig) -> Result<reqwest::Client, reqwest::Error> {
        if let Some(client) = &*self.client.read().await {
            Ok(client.clone())
        } else {
            let mut state_client = self.client.write().await;

            if let Some(client) = &*state_client {
                // another thread may have created a client since we last check, if so, use that.
                Ok(client.clone())
            } else {
                // else, create a client. Return an error if that fails
                let client = get_http_client(&config.connection)?;
                // store a copy of the new client
                *state_client = Some(client.clone());

                Ok(client)
            }
        }
    }
}
