//! NexusFlow 会话管理
//!
//! 会话生命周期管理、PTY 终端模拟和检查点/恢复支持。

pub mod session;
pub mod manager;
pub mod checkpoint;
pub mod persistence;
pub mod pty;

pub use session::{Session, SessionId, SessionStatus, SessionState, SessionMetadata};
pub use manager::SessionManager;
pub use checkpoint::{Checkpoint, CheckpointManager};
pub use persistence::{SessionStore, PersistenceError};
pub use pty::{PtyManager, PtySession, PtySessionState, PtyOutput, PtyError};
