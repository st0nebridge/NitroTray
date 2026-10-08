//! @module cu_20261001_001
//! @description Regression test (popup too tall on its first open): a borderless tool window must have exactly the
//! requested client size the first time it is shown. Before its frame was settled, the first show came
//! out ~30 px (a caption) taller than the size its placement was computed for, overlapping the taskbar.
use tao::dpi::PhysicalPosition;
use tao::event_loop::EventLoopBuilder;
use tao::platform::windows::{EventLoopBuilderExtWindows, WindowExtWindows};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

use nitrotray::ui::flyout::FLYOUT_SIZE;
use nitrotray::ui::webview::{hide_window, show_inactive, tool_window, POPUP_SIZE};

#[test]
fn first_show_has_the_requested_client_size() {
    let event_loop = EventLoopBuilder::<()>::new().with_any_thread(true).build();
    for (name, size, focused) in [("popup", POPUP_SIZE, true), ("flyout", FLYOUT_SIZE, false)] {
        let window = tool_window(&event_loop, size, false, focused).expect("window");
        // Off-screen, shown without activation: nothing appears on the desktop or takes focus.
        window.set_outer_position(PhysicalPosition::new(-32000, -32000));
        let hwnd = HWND(window.hwnd() as *mut _);
        show_inactive(hwnd);
        let mut client = RECT::default();
        // SAFETY: reads the client rectangle of a window this test owns.
        unsafe { GetClientRect(hwnd, &mut client) }.expect("client rect");
        let scale = window.scale_factor();
        hide_window(hwnd);
        let want = ((size.0 * scale).round() as i32, (size.1 * scale).round() as i32);
        assert_eq!((client.right, client.bottom), want, "{name}: first show at scale {scale}");
    }
}
