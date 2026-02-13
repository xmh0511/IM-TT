use redis::{Client, aio::MultiplexedConnection};
use anyhow::Result;

pub async fn create_redis_client(redis_url: &str) -> Result<Client> {
    let client = Client::open(redis_url)?;
    Ok(client)
}

pub async fn get_redis_connection(client: &Client) -> Result<MultiplexedConnection> {
    let conn = client.get_multiplexed_tokio_connection().await?;
    Ok(conn)
}
