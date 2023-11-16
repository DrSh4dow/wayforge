mod client_data;
mod loop_shared_data;
mod state;

use anyhow::Context;
use std::{ffi::OsString, sync::Arc};

use loop_shared_data::LoopSharedData;
use smithay::{
    backend::{renderer::gles::GlesRenderer, winit},
    reexports::{
        calloop::{generic::Generic, EventLoop, Interest, LoopHandle, Mode, PostAction},
        wayland_server::{Display, DisplayHandle},
    },
    wayland::socket::ListeningSocketSource,
};
use state::WayforgeState;

pub fn init_wayforge() -> anyhow::Result<(), anyhow::Error> {
    // initialize the event loop
    let event_loop: EventLoop<LoopSharedData> =
        EventLoop::try_new().expect("Failed to initialize the event loop.");

    let event_loop_handle = event_loop.handle();

    let (display_handle, socket_name) = init_wayland_display(&event_loop_handle)?;

    let _wayforge_state = WayforgeState::new(&display_handle, socket_name, event_loop_handle);

    let (mut backend, mut winit) = winit::init::<GlesRenderer>().unwrap();


    Ok(())
}

/// initialize the display server and add it to the event loop
fn init_wayland_display(
    event_loop_handle: &LoopHandle<LoopSharedData>,
) -> anyhow::Result<(DisplayHandle, OsString)> {
    let display: Display<WayforgeState> =
        Display::new().expect("Failed to initialize display server");
    let display_handle = display.handle();

    //////////////////// ADD BASIC SOURCES ////////////////////

    // initialize the socket source
    let socket =
        ListeningSocketSource::new_auto().context("Failed to get a new listening socket source")?;
    let socket_name = socket.socket_name().to_os_string();

    // adds wayland clients to the wayland server
    event_loop_handle
        .insert_source(socket, |client_stream, _, shared_data| {
            if let Err(err) = shared_data
                .wayforge_state
                .display_handle
                .insert_client(client_stream, Arc::new(client_data::ClientData::default()))
            {
                tracing::warn!(?err, "Error adding wayland client");
            };
        })
        .with_context(|| "Failed to init the wayland socket source.")?;

    // adds the display itself to the event loop as a source
    event_loop_handle
        .insert_source(
            Generic::new(display, Interest::READ, Mode::Level),
            move |_, display, state| {
                // SAFETY: We don't drop the display
                match unsafe {
                    display
                        .get_mut()
                        .dispatch_clients(&mut state.wayforge_state)
                } {
                    Ok(_) => Ok(PostAction::Continue),
                    Err(err) => {
                        tracing::error!(?err, "I/O error on the Wayland display");
                        Err(err)
                    }
                }
            },
        )
        .with_context(|| "Failed to init the wayland event source")?;

    Ok((display_handle, socket_name))
}
