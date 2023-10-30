use smbioslib::table_load_from_device;

fn main() {
    let data = table_load_from_device().unwrap();
    let sysinfo = data
        .find_map(|sysinfo: smbioslib::SMBiosBaseboardInformation| {
            Some(sysinfo)
        })
        .unwrap();
    println!("{:#?}", sysinfo);
}
