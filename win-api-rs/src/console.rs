use windows::core::Result;
use windows::Win32::System::Console::{
    GetConsoleWindow, GetStdHandle, SetConsoleMode, ENABLE_EXTENDED_FLAGS,
    ENABLE_QUICK_EDIT_MODE, STD_INPUT_HANDLE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DrawMenuBar, EnableMenuItem, GetSystemMenu, ShowWindow, MF_ENABLED,
    MF_GRAYED, SC_CLOSE, SW_HIDE, SW_RESTORE,
};

pub fn enable_quick_edit(enable: bool) -> Result<()> {
    unsafe {
        let stdin = GetStdHandle(STD_INPUT_HANDLE)?;
        SetConsoleMode(
            stdin,
            if enable {
                ENABLE_QUICK_EDIT_MODE
            } else {
                ENABLE_EXTENDED_FLAGS
            },
        )?;
        Ok(())
    }
}

pub fn set_close_button(show: bool) -> Result<()> {
    unsafe {
        EnableMenuItem(
            GetSystemMenu(GetConsoleWindow(), false),
            SC_CLOSE,
            if show { MF_ENABLED } else { MF_GRAYED },
        );
        DrawMenuBar(GetConsoleWindow())?;
        Ok(())
    }
}

pub fn hide_window(enable: bool) -> Result<()> {
    unsafe {
        ShowWindow(
            GetConsoleWindow(),
            if enable { SW_RESTORE } else { SW_HIDE },
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn console_test() {
        println!("disable quick edit");
        enable_quick_edit(false).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));

        println!("disable close button");
        set_close_button(false).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));

        /* println!("hide window");
        hide_window(true).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));

        println!("show window");
        hide_window(false).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1)); */

        set_close_button(true).unwrap();
        enable_quick_edit(true).unwrap();
    }
}
