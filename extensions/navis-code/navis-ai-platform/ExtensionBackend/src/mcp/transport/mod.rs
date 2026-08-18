//! MCP 传输扩展接口。

pub trait TransportAdapter: Send + Sync {
    fn protocol(&self) -> &str;
}
