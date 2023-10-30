use winreg::enums::*;
use winreg::RegKey;

fn main() -> std::io::Result<()> {
    let hklmm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let cur_ver =
        hklmm.open_subkey("SOFTWARE\\Microsoft\\Windows\\CurrentVersion")?;
    let pf = cur_ver.get_value::<String, _>("ProgramFilesDir")?;
    let dp = cur_ver.get_value::<String, _>("DevicePath")?;
    println!("ProgramFilesDir: {}", pf);
    println!("DevicePath: {}", dp);

    let info = cur_ver.query_info()?;
    println!("Info: {:?}", info);
    Ok(())
}
