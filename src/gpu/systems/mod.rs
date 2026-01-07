// ============================================
// Systems Module - ECS-подобные системы
// ============================================

mod input_system;
mod block_interaction_system;
mod menu_system;
mod save_system;
mod update_system;
mod render_system;
mod init_system;

pub use input_system::{InputSystem, InputAction};
pub use block_interaction_system::BlockInteractionSystem;
pub use menu_system::MenuSystem;
pub use save_system::SaveSystem;
pub use update_system::UpdateSystem;
pub use render_system::RenderSystem;
pub use init_system::InitSystem;
