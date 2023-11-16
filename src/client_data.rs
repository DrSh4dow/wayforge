use smithay::{reexports::wayland_server::backend, wayland::compositor::CompositorClientState};

// Used to store client data associated with Wayland clients
#[derive(Default)]
pub struct ClientData {
    pub compositor_client_state: CompositorClientState,
}

impl backend::ClientData for ClientData {}
