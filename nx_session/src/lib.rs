//! NexusFlow 会话管理
//!
//! 会话生命周期管理、PTY 终端模拟和检查点/恢复支持。

pub mod checkpoint;
pub mod manager;
pub mod persistence;
pub mod pty;
pub mod session;

pub use checkpoint::{Checkpoint, CheckpointManager};
pub use manager::SessionManager;
pub use persistence::{PersistenceError, SessionStore};
pub use pty::{PtyError, PtyManager, PtyOutput, PtySession, PtySessionState};
pub use session::{Session, SessionId, SessionMetadata, SessionState, SessionStatus};
