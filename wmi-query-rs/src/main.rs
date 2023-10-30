#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use serde::Deserialize;
use std::collections::HashMap;

use wmi::{COMLibrary, Variant, WMIConnection};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let com_con = COMLibrary::new()?;
    let wmi_con = WMIConnection::new(com_con)?;

    let results: Vec<HashMap<String, Variant>> =
        wmi_con.raw_query("select PnPDeviceID from Win32_VideoController")?;
    for r in results {
        println!("{:#?}", r);
    }

    #[derive(Deserialize, Debug)]
    struct Win32_VideoController {
        PnPDeviceID: String,
    }
    let result: Vec<Win32_VideoController> = wmi_con.query()?;
    for r in result {
        println!("{:#?}", r);
        println!("{}", r.PnPDeviceID);
    }
    Ok(())
}
