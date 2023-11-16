use std::{ffi::OsString, os::fd::OwnedFd};

use smithay::{
    backend::renderer::utils::on_commit_buffer_handler,
    delegate_compositor, delegate_data_device, delegate_output, delegate_seat, delegate_shm,
    delegate_xdg_shell,
    desktop::{Space, Window},
    input::{pointer::CursorImageStatus, Seat, SeatHandler, SeatState},
    reexports::{
        calloop::LoopHandle,
        wayland_protocols::xdg::shell::client::xdg_toplevel,
        wayland_server::{
            protocol::{wl_seat, wl_shm, wl_surface::WlSurface},
            Client, DisplayHandle,
        },
    },
    utils::{Clock, Logical, Monotonic, Point, Serial},
    wayland::{
        buffer::BufferHandler,
        compositor::{with_states, CompositorClientState, CompositorHandler, CompositorState},
        input_method::PopupSurface,
        output::OutputManagerState,
        selection::{
            data_device::{
                ClientDndGrabHandler, DataDeviceHandler, DataDeviceState, ServerDndGrabHandler,
            },
            SelectionHandler, SelectionSource, SelectionTarget,
        },
        shell::xdg::{
            PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
            XdgToplevelSurfaceData,
        },
        shm::{ShmHandler, ShmState},
    },
    xwayland::{xwm::XwmId, XWaylandClientData},
};

use crate::{client_data::ClientData, loop_shared_data::LoopSharedData};

pub struct WayforgeState {
    // extra data to tune the compositor
    pub clock: Clock<Monotonic>,

    // handlers
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

impl WayforgeState {
    pub fn new(
        dh: &DisplayHandle,
        socket: OsString,
        event_loop_handle: LoopHandle<LoopSharedData>,
    ) -> Self {
        let clock = Clock::new();
        let compositor_state = CompositorState::new::<Self>(dh);

        // shared memory state, used to share surfaces buffers with clients
        let shm_state =
            ShmState::new::<Self>(dh, vec![wl_shm::Format::Xbgr8888, wl_shm::Format::Abgr8888]);
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(dh);
        let xdg_shell_state = XdgShellState::new::<Self>(dh);
        let mut seat_state = SeatState::<Self>::new();

        // A space to map windows on. Keeps track of windows and outputs, can access either with
        // space.elements() and space.outputs().
        let space = Space::<Window>::default();

        // Manage copy/paste and drag-and-drop from inputs.
        let data_device_state = DataDeviceState::new::<Self>(dh);

        // Create a new seat from the seat state, we pass in a name .
        let mut seat: Seat<Self> = seat_state.new_wl_seat(dh, "mwm_seat");
        // Add a keyboard with repeat rate and delay in milliseconds. The repeat is the time to
        // repeat, then delay is how long to wait until the next repeat.
        seat.add_keyboard(Default::default(), 300, 60);
        // Add pointer to seat.
        seat.add_pointer();

        todo!();
    }
}

impl BufferHandler for WayforgeState {
    fn buffer_destroyed(
        &mut self,
        buffer: &smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer,
    ) {
    }
}

pub fn client_compositor_state<'a>(client: &'a Client) -> &'a CompositorClientState {
    if let Some(state) = client.get_data::<XWaylandClientData>() {
        return &state.compositor_state;
    }
    if let Some(state) = client.get_data::<ClientData>() {
        return &state.compositor_client_state;
    }
    panic!("Unknown client data type")
}

impl CompositorHandler for WayforgeState {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    // Called on every buffer commit in Wayland to update a surface. This has the new state of the
    // surface.
    fn commit(&mut self, surface: &WlSurface) {
        // Let Smithay take the surface buffer so that desktop helpers get the new surface state.
        on_commit_buffer_handler::<Self>(surface);

        // Find the window with the xdg toplevel surface to update.
        if let Some(window) = self
            .space
            .elements()
            .find(|w| w.toplevel().wl_surface() == surface)
            .cloned()
        {
            // Refresh the window state.
            window.on_commit();

            // Find if the window has been configured yet.
            let initial_configure_sent = with_states(surface, |states| {
                states
                    .data_map
                    .get::<XdgToplevelSurfaceData>()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .initial_configure_sent
            });

            if !initial_configure_sent {
                // Configure window size/attributes.
                window.toplevel().send_pending_configure();
            }
        }
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        client_compositor_state(client)
    }
}
delegate_compositor!(WayforgeState);

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

impl ClientDndGrabHandler for WayforgeState {}
impl ServerDndGrabHandler for WayforgeState {}


impl DataDeviceHandler for WayforgeState {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}
delegate_data_device!(WayforgeState);

impl ShmHandler for WayforgeState {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}
delegate_shm!(WayforgeState);

impl XdgShellHandler for WayforgeState {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let window = Window::new(surface);

        // Add the window to the space so we can use it elsewhere in our application, such as the
        // CompositorHandler.
        self.workspaces
            .insert_window(self.workspaces.active(), window.clone());
        self.space.map_element(window, (0, 0), false);

        // Resize and reposition all the windows.
        self.workspaces.refresh_geometry(&mut self.space);
    }

    fn toplevel_destroyed(&mut self, _: ToplevelSurface) {
        self.workspaces.refresh_geometry(&mut self.space);

        if self.workspaces.is_workspace_empty(self.workspaces.active())
            && self.cursor_status != CursorImageStatus::Default
        {
            self.cursor_status = CursorImageStatus::Default;
        }
    }

    fn new_popup(&mut self, _: PopupSurface, _: PositionerState) {}

    fn move_request(&mut self, _: ToplevelSurface, _: wl_seat::WlSeat, _: Serial) {}

    fn resize_request(
        &mut self,
        _: ToplevelSurface,
        _: wl_seat::WlSeat,
        _: Serial,
        _: xdg_toplevel::ResizeEdge,
    ) {
    }

    fn grab(&mut self, _surface: PopupSurface, _seat: wl_seat::WlSeat, _serial: Serial) {}
}
delegate_xdg_shell!(WayforgeState);

delegate_output!(WayforgeState);
