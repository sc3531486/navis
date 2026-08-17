pub trait AgentControlPorts: Send + Sync {}
pub trait SidechainPort: Send + Sync {}
pub trait TodoPort: Send + Sync {}
pub struct SidechainStartRequest;
pub struct SidechainStarted;
pub struct SidechainReadRequest;
pub struct SidechainTaskSnapshot;
pub enum SidechainStatus { Running, Completed, Failed }
pub struct TodoUpdate;
pub struct TodoUpdateRequest;
