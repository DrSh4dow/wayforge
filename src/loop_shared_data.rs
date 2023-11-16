use smithay::reexports::wayland_server::Display;

use crate::state::WayforgeState;

pub struct LoopSharedData {
    pub display: Display<WayforgeState>,
    pub wayforge_state: WayforgeState,
}
