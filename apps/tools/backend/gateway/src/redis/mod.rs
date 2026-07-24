pub mod job;
pub mod ratelimit;

use redis::Client;

/// Create a Redis client.
pub fn create_client(url: &str) -> Result<Client, redis::RedisError> {
    Client::open(url)
}