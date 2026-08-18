//! Service trait
//! 参考 DeepSeek Harness Cordis Service，通用框架，不绑定业务领域。

pub trait Service: Send {
    fn name(&self) -> &str;
    fn dependencies(&self) -> Vec<&str> { vec![] }
    fn start(&mut self) -> Result<(), String> { Ok(()) }
    fn stop(&mut self) -> Result<(), String> { Ok(()) }
}
