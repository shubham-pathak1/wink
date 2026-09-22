#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(not(windows))]
fn main() {
    eprintln!("Wink currently supports Windows only.");
}

#[cfg(windows)]
mod windows_app {
    use std::mem::{size_of, zeroed};
    use std::ptr::{null, null_mut};

    use windows_sys::Win32::{
        Foundation::{
            CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, POINT, WPARAM,
        },
        System::{
            LibraryLoader::GetModuleHandleW,
            Power::{
                SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED,
            },
            Threading::CreateMutexW,
        },
        UI::{
            Shell::{
                Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
                NOTIFYICONDATAW,
            },
            WindowsAndMessaging::{
                AppendMenuW, CreateIconFromResourceEx, CreatePopupMenu, CreateWindowExW,
                DefWindowProcW, DestroyIcon, DestroyMenu, DispatchMessageW, GetCursorPos,
                GetMessageW, LoadIconW, PostQuitMessage, RegisterClassW, SetForegroundWindow,
                TrackPopupMenu, TranslateMessage, HICON, HWND_MESSAGE, IDI_APPLICATION, MF_CHECKED,
                MF_STRING, MF_UNCHECKED, MSG, TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_RIGHTBUTTON,
                WM_APP, WM_COMMAND, WM_DESTROY, WM_LBUTTONUP, WM_RBUTTONUP, WNDCLASSW,
            },
        },
    };

    const TRAY_CALLBACK: u32 = WM_APP + 1;
    const ID_STAY_AWAKE: usize = 1;
    const ID_ALLOW_SLEEP: usize = 2;
    const ID_QUIT: usize = 3;
    const CLASS_NAME: &[u16] = &[87, 105, 110, 107, 84, 114, 97, 121, 0];
    const INSTANCE_MUTEX: &[u16] = &[
        76, 111, 99, 97, 108, 92, 87, 105, 110, 107, 46, 83, 105, 110, 103, 108, 101, 73, 110, 115,
        116, 97, 110, 99, 101, 0,
    ];
    const TOOLTIP: &[u16] = &[
        87, 105, 110, 107, 58, 32, 75, 101, 101, 112, 105, 110, 103, 32, 121, 111, 117, 114, 32,
        80, 67, 32, 97, 119, 97, 107, 101, 0,
    ];
    const STAY_AWAKE: &[u16] = &[83, 116, 97, 121, 32, 97, 119, 97, 107, 101, 0];
    const ALLOW_SLEEP: &[u16] = &[65, 108, 108, 111, 119, 32, 115, 108, 101, 101, 112, 0];
    const QUIT: &[u16] = &[81, 117, 105, 116, 0];

    const ICON_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/wink.ico"));

    static mut KEEP_AWAKE: bool = true;
    static mut TRAY_ICON: HICON = null_mut();

    pub fn run() {
        unsafe {
            let instance_lock = CreateMutexW(null(), 0, INSTANCE_MUTEX.as_ptr());
            if instance_lock.is_null() || GetLastError() == ERROR_ALREADY_EXISTS {
                if !instance_lock.is_null() {
                    CloseHandle(instance_lock);
                }
                return;
            }

            set_keep_awake(true);

            let instance = GetModuleHandleW(null());
            let window_class = WNDCLASSW {
                lpfnWndProc: Some(window_proc),
                hInstance: instance,
                lpszClassName: CLASS_NAME.as_ptr(),
                ..zeroed()
            };
            RegisterClassW(&window_class);

            // A message-only window receives tray callbacks but is never visible.
            let window = CreateWindowExW(
                0,
                CLASS_NAME.as_ptr(),
                CLASS_NAME.as_ptr(),
                0,
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                null_mut(),
                instance,
                null(),
            );

            if window.is_null() {
                set_keep_awake(false);
                CloseHandle(instance_lock);
                return;
            }

            let mut icon = notification_data(window);
            if Shell_NotifyIconW(NIM_ADD, &mut icon) == 0 {
                DestroyIcon(TRAY_ICON);
                TRAY_ICON = null_mut();
                set_keep_awake(false);
                CloseHandle(instance_lock);
                return;
            }

            let mut message: MSG = zeroed();
            while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }

            CloseHandle(instance_lock);
        }
    }

    unsafe fn notification_data(window: HWND) -> NOTIFYICONDATAW {
        let mut data: NOTIFYICONDATAW = zeroed();
        data.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
        data.hWnd = window;
        data.uID = 1;
        data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        data.uCallbackMessage = TRAY_CALLBACK;
        data.hIcon = tray_icon();
        data.szTip[..TOOLTIP.len()].copy_from_slice(TOOLTIP);
        data
    }

    unsafe fn tray_icon() -> HICON {
        if !TRAY_ICON.is_null() {
            return TRAY_ICON;
        }

        let entry_count = u16::from_le_bytes([ICON_BYTES[4], ICON_BYTES[5]]) as usize;
        for index in 0..entry_count {
            let entry = 6 + index * 16;
            if entry + 16 > ICON_BYTES.len() {
                break;
            }

            let width = if ICON_BYTES[entry] == 0 {
                256
            } else {
                ICON_BYTES[entry] as i32
            };
            if width != 16 {
                continue;
            }

            let length = u32::from_le_bytes(ICON_BYTES[entry + 8..entry + 12].try_into().unwrap());
            let offset =
                u32::from_le_bytes(ICON_BYTES[entry + 12..entry + 16].try_into().unwrap()) as usize;
            let end = offset.saturating_add(length as usize);
            if end > ICON_BYTES.len() {
                break;
            }

            TRAY_ICON = CreateIconFromResourceEx(
                ICON_BYTES[offset..end].as_ptr(),
                length,
                1,
                0x0003_0000,
                0,
                0,
                0,
            );
            if !TRAY_ICON.is_null() {
                return TRAY_ICON;
            }
        }

        LoadIconW(null_mut(), IDI_APPLICATION) as HICON
    }

    unsafe fn set_keep_awake(enabled: bool) {
        KEEP_AWAKE = enabled;
        let state = if enabled {
            ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED
        } else {
            ES_CONTINUOUS
        };
        SetThreadExecutionState(state);
    }

    unsafe extern "system" fn window_proc(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            TRAY_CALLBACK if lparam as u32 == WM_RBUTTONUP || lparam as u32 == WM_LBUTTONUP => {
                show_menu(window);
                0
            }
            WM_COMMAND => {
                match wparam & 0xffff {
                    ID_STAY_AWAKE => set_keep_awake(true),
                    ID_ALLOW_SLEEP => set_keep_awake(false),
                    ID_QUIT => {
                        let mut icon = notification_data(window);
                        Shell_NotifyIconW(NIM_DELETE, &mut icon);
                        set_keep_awake(false);
                        DestroyIcon(TRAY_ICON);
                        TRAY_ICON = null_mut();
                        PostQuitMessage(0);
                    }
                    _ => {}
                }
                0
            }
            WM_DESTROY => {
                set_keep_awake(false);
                0
            }
            _ => DefWindowProcW(window, message, wparam, lparam),
        }
    }

    unsafe fn show_menu(window: HWND) {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return;
        }

        let stay_awake_state = if KEEP_AWAKE { MF_CHECKED } else { MF_UNCHECKED };
        let allow_sleep_state = if KEEP_AWAKE { MF_UNCHECKED } else { MF_CHECKED };
        AppendMenuW(
            menu,
            MF_STRING | stay_awake_state,
            ID_STAY_AWAKE,
            STAY_AWAKE.as_ptr(),
        );
        AppendMenuW(
            menu,
            MF_STRING | allow_sleep_state,
            ID_ALLOW_SLEEP,
            ALLOW_SLEEP.as_ptr(),
        );
        AppendMenuW(menu, MF_STRING, ID_QUIT, QUIT.as_ptr());

        let mut point: POINT = zeroed();
        GetCursorPos(&mut point);
        SetForegroundWindow(window);
        TrackPopupMenu(
            menu,
            TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            0,
            window,
            null(),
        );
        DestroyMenu(menu);
    }
}

#[cfg(windows)]
fn main() {
    windows_app::run();
}
