use smithay::{
    delegate_seat,
    desktop::{Space, Window},
    input::{pointer::CursorImageStatus, SeatHandler, SeatState},
    reexports::wayland_server::{protocol::wl_surface::WlSurface, DisplayHandle},
    utils::{Logical, Point, Clock, Monotonic},
    wayland::{
        compositor::CompositorState, output::OutputManagerState,
        selection::data_device::DataDeviceState, shell::xdg::XdgShellState, shm::ShmState,
    },
};

pub struct WayforgeState {
    // extra data needed to tune the compositor -- not required by smithay
    pub clock: Clock<Monotonic>,
    pub display_handle: DisplayHandle,

    // minimal data needed to run the compositor -- required by smithay
    pub compositor_state: CompositorState,
    pub data_device_state: DataDeviceState,
    pub seat_state: SeatState<Self>,
    pub shm_state: ShmState,
    pub space: Space<Window>,
    pub cursor_status: CursorImageStatus,
    pub pointer_location: Point<f64, Logical>,
    pub output_manager_state: OutputManagerState,
    pub xdg_shell_state: XdgShellState,
}

impl SeatHandler for WayforgeState {
    type KeyboardFocus = WlSurface;

    type PointerFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn cursor_image(&mut self, _seat: &smithay::input::Seat<Self>, image: CursorImageStatus) {
        self.cursor_status = image;
    }
}
delegate_seat!(WayforgeState);
