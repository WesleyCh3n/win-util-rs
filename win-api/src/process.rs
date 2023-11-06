use std::collections::{HashMap, HashSet, VecDeque};

use std::cell::RefCell;
use std::rc::Rc;

use windows::Wdk::System::Threading::{
    NtQueryInformationProcess, PROCESSINFOCLASS,
};

use windows::core::{HRESULT, PSTR, PWSTR};
use windows::Win32::System::{
    Diagnostics::{
        Debug::ReadProcessMemory,
        ToolHelp::{
            CreateToolhelp32Snapshot, Process32First, Process32Next,
            PROCESSENTRY32, TH32CS_SNAPPROCESS,
        },
    },
    Threading::{
        OpenProcess, PEB, PROCESS_BASIC_INFORMATION, PROCESS_QUERY_INFORMATION,
        PROCESS_VM_READ, RTL_USER_PROCESS_PARAMETERS,
    },
};

#[derive(Debug, Default)]
pub struct ProcessInfo {
    name: String,
    path: String,
    arg: String,
}

#[derive(Debug, Default)]
pub struct PidNode {
    pid: u32,
    // name: String,
    process_info: ProcessInfo,
    children: Vec<Rc<RefCell<PidNode>>>,
}

impl PidNode {
    fn new(pid: u32, process_info: ProcessInfo) -> PidNode {
        PidNode {
            pid,
            process_info,
            children: Vec::new(),
        }
    }
}

#[derive(Debug, Default)]
pub struct PrintConfig {
    show_pid: bool,
    show_arg: bool,
    show_path: bool,
    sort: bool,
}

impl PrintConfig {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn show_pid(mut self, show_pid: bool) -> Self {
        self.show_pid = show_pid;
        self
    }
    pub fn show_arg(mut self, show_arg: bool) -> Self {
        self.show_arg = show_arg;
        self
    }
    pub fn show_path(mut self, show_path: bool) -> Self {
        self.show_path = show_path;
        self
    }
    pub fn sort(mut self, sort: bool) -> Self {
        self.sort = sort;
        self
    }
    pub fn build(self) -> Self {
        self
    }
}

pub fn print(root: Rc<RefCell<PidNode>>, cfg: &PrintConfig) {
    dfs(root, Vec::new(), false, cfg);
}

fn dfs(
    root: Rc<RefCell<PidNode>>,
    mut prefix: Vec<char>,
    have_siblings: bool,
    cfg: &PrintConfig,
) {
    println!(
        "{}{} {} {}",
        prefix.iter().collect::<String>(),
        if cfg.show_pid {
            format!("({})", root.borrow().pid)
        } else {
            String::default()
        },
        if cfg.show_path {
            root.borrow().process_info.path.clone()
        } else {
            root.borrow().process_info.name.clone()
        },
        if cfg.show_arg {
            root.borrow().process_info.arg.clone()
        } else {
            String::default()
        }
    );
    {
        let children = &root.borrow_mut().children;
        // if I have siblings and i have child
        if have_siblings && !children.is_empty() {
            prefix.truncate(prefix.len() - 4);
            prefix.append(&mut vec![' ', '║', ' ', ' ']);
        }
        // if I don't have siblings and i have child
        if !have_siblings && !children.is_empty() && prefix.len() >= 4 {
            prefix.truncate(prefix.len() - 4);
            prefix.append(&mut vec![' ', ' ', ' ', ' ']);
        }
        // if I have child?
        if !children.is_empty() {
            prefix.append(&mut vec![' ', '╠', '═', ' ']);
        }
    }
    if cfg.sort {
        root.borrow_mut()
            .children
            .sort_by(|a, b| a.borrow().pid.cmp(&b.borrow().pid));
    }
    for (i, child) in root.borrow().children.iter().enumerate() {
        let last_child = i == root.borrow().children.len() - 1;
        if last_child {
            prefix.truncate(prefix.len() - 4);
            prefix.append(&mut vec![' ', '╚', '═', ' ']);
        }
        dfs(child.clone(), prefix.clone(), !last_child, cfg);
    }
}

fn find_child(
    root: Rc<RefCell<PidNode>>,
    pid: u32,
) -> Option<Rc<RefCell<PidNode>>> {
    if root.borrow().pid == pid {
        return Some(root);
    }
    if root.borrow().children.is_empty() {
        return None;
    }
    let mut queue = VecDeque::new();
    for child in root.borrow().children.iter() {
        queue.push_back(Rc::clone(child));
    }
    while let Some(node) = queue.pop_front() {
        if node.borrow().pid == pid {
            return Some(node);
        }
        for child in node.borrow().children.iter() {
            queue.push_back(Rc::clone(child));
        }
    }
    None
}

pub fn add_parent(root: Rc<RefCell<PidNode>>) -> Rc<RefCell<PidNode>> {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).unwrap();
        let mut entry = PROCESSENTRY32 {
            dwSize: std::mem::size_of::<PROCESSENTRY32>() as u32,
            ..Default::default()
        };
        let mut mappid = HashMap::new();
        let mut mapname = HashMap::new();
        loop {
            mappid.insert(entry.th32ProcessID, entry.th32ParentProcessID);
            mapname.insert(
                entry.th32ProcessID,
                PSTR::from_raw(entry.szExeFile.as_mut_ptr())
                    .to_string()
                    .unwrap(),
            );
            if Process32Next(snapshot, &mut entry as *mut PROCESSENTRY32)
                .is_err()
            {
                break;
            }
        }
        let mut head = Rc::clone(&root);
        loop {
            let pid = head.borrow().pid;
            if let Some(parent_pid) = mappid.get(&pid) {
                if let Some(parent_name) = mapname.get(parent_pid) {
                    let process_info = ProcessInfo {
                        name: parent_name.clone(),
                        ..Default::default()
                    };

                    let new_head = Rc::new(RefCell::new(PidNode::new(
                        *parent_pid,
                        process_info,
                    )));
                    new_head.borrow_mut().children = vec![Rc::clone(&head)];
                    head = new_head;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        head
    }
}

pub fn find_by_pid(pid: u32) -> Option<Rc<RefCell<PidNode>>> {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).unwrap();
        let mut entry = PROCESSENTRY32 {
            dwSize: std::mem::size_of::<PROCESSENTRY32>() as u32,
            ..Default::default()
        };
        let mut tree = None;
        // find parent first
        Process32First(snapshot, &mut entry as *mut PROCESSENTRY32).unwrap();
        loop {
            if entry.th32ProcessID == pid {
                let mut process_info = ProcessInfo {
                    name: PSTR::from_raw(entry.szExeFile.as_mut_ptr())
                        .to_string()
                        .unwrap(),
                    ..Default::default()
                };
                get_proc_info(pid, &mut process_info).unwrap();

                tree = Some(Rc::new(RefCell::new(PidNode::new(
                    pid,
                    process_info,
                ))));
                break;
            }
            if Process32Next(snapshot, &mut entry as *mut PROCESSENTRY32)
                .is_err()
            {
                break;
            }
        }

        // if parent exsit
        Process32First(snapshot, &mut entry as *mut PROCESSENTRY32).unwrap();
        if let Some(parent) = tree.as_ref() {
            loop {
                // iterate tree and add child
                if let Some(node) =
                    find_child(parent.clone(), entry.th32ParentProcessID)
                {
                    let mut process_info = ProcessInfo {
                        name: PSTR::from_raw(entry.szExeFile.as_mut_ptr())
                            .to_string()
                            .unwrap(),
                        ..Default::default()
                    };
                    get_proc_info(pid, &mut process_info).unwrap();
                    node.borrow_mut().children.push(Rc::new(RefCell::new(
                        PidNode::new(entry.th32ProcessID, process_info),
                    )));
                }
                if Process32Next(snapshot, &mut entry as *mut PROCESSENTRY32)
                    .is_err()
                {
                    break;
                }
            }
        }
        tree
    }
}

pub fn list_all() -> Vec<Rc<RefCell<PidNode>>> {
    let mut processes = Vec::new();
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).unwrap();
        let mut entry = PROCESSENTRY32 {
            dwSize: std::mem::size_of::<PROCESSENTRY32>() as u32,
            ..Default::default()
        };
        // find root ids (process with no parent)
        let mut proc_ids = HashSet::new();
        let mut all_proc = Vec::new();
        let mut root_ids = HashSet::new();
        Process32First(snapshot, &mut entry as *mut PROCESSENTRY32).unwrap();
        loop {
            proc_ids.insert(entry.th32ProcessID);
            all_proc.push((entry.th32ParentProcessID, entry.th32ProcessID));
            if Process32Next(snapshot, &mut entry as *mut PROCESSENTRY32)
                .is_err()
            {
                break;
            }
        }
        // find the process that its parent not exist in parent_ids
        for (parent_id, proc_id) in all_proc {
            if !proc_ids.contains(&parent_id) {
                root_ids.insert(proc_id);
            }
        }
        root_ids.insert(0);
        let mut root_ids: Vec<u32> = root_ids.into_iter().collect();
        root_ids.sort();
        for root_id in root_ids {
            processes.push(find_by_pid(root_id).unwrap());
        }
    }
    processes
}

pub fn get_proc_info(
    pid: u32,
    process_info: &mut ProcessInfo,
) -> windows::core::Result<()> {
    unsafe {
        if pid == 0 {
            return Ok(());
        }
        let h_process = match OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
            false,
            pid,
        ) {
            Ok(h) => h,
            Err(e) => {
                if e.code() == HRESULT(0x80070005u32 as i32) {
                    // Access is denied.
                    process_info.path = e.message().to_string();
                    process_info.arg = e.message().to_string();
                    return Ok(());
                }
                return Err(e);
            }
        };
        // get process basic information
        let pic = std::mem::zeroed::<PROCESSINFOCLASS>();
        let mut pbi =
            std::mem::MaybeUninit::<PROCESS_BASIC_INFORMATION>::uninit();
        NtQueryInformationProcess(
            h_process,
            pic,
            pbi.as_mut_ptr() as *mut std::ffi::c_void,
            std::mem::size_of::<PROCESS_BASIC_INFORMATION>() as u32,
            std::ptr::null_mut(),
        )?;
        // get peb
        let pbi = pbi.assume_init();
        let mut peb = std::mem::MaybeUninit::<PEB>::uninit();
        ReadProcessMemory(
            h_process,
            pbi.PebBaseAddress as *const std::ffi::c_void,
            peb.as_mut_ptr() as *mut std::ffi::c_void,
            std::mem::size_of::<PEB>(),
            Some(std::ptr::null_mut()),
        )?;
        // get process parameters
        let peb = peb.assume_init();
        let mut proc_params =
            std::mem::MaybeUninit::<RTL_USER_PROCESS_PARAMETERS>::uninit();
        ReadProcessMemory(
            h_process,
            peb.ProcessParameters as *const std::ffi::c_void,
            proc_params.as_mut_ptr() as *mut std::ffi::c_void,
            std::mem::size_of::<RTL_USER_PROCESS_PARAMETERS>(),
            Some(std::ptr::null_mut()),
        )?;
        // get command line
        let proc_params = proc_params.assume_init();
        let cmdline_len = proc_params.CommandLine.Length as usize;
        let mut cmdline: Vec<u16> = vec![0; cmdline_len];
        ReadProcessMemory(
            h_process,
            proc_params.CommandLine.Buffer.as_ptr() as _,
            cmdline.as_mut_ptr() as *mut std::ffi::c_void,
            cmdline_len,
            Some(std::ptr::null_mut()),
        )?;
        process_info.arg = PWSTR::from_raw(cmdline.as_mut_ptr()).to_string()?;
        // get image path
        let img_path_len = proc_params.ImagePathName.Length as usize;
        let mut img_path: Vec<u16> = vec![0; img_path_len];
        ReadProcessMemory(
            h_process,
            proc_params.ImagePathName.Buffer.as_ptr() as _,
            img_path.as_mut_ptr() as *mut std::ffi::c_void,
            img_path_len,
            Some(std::ptr::null_mut()),
        )?;
        process_info.path =
            PWSTR::from_raw(img_path.as_mut_ptr()).to_string()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_test() {
        let tree = find_by_pid(7980);
        if let Some(tree) = tree {
            println!("{: <6}  Process Name", "PID");
            let cfg = PrintConfig::new()
                .show_pid(true)
                .sort(false)
                .show_arg(true)
                .build();

            print(tree, &cfg);
        }
    }

    #[test]
    fn list_test() {
        let processes = list_all();
        println!("{: <6}  Process Name", "PID");
        let cfg = PrintConfig::new()
            .show_pid(true)
            .sort(true)
            .show_arg(true)
            .build();

        for process in processes {
            print(process, &cfg);
        }
    }

    #[test]
    fn proc_info_test() {
        let mut process_info = ProcessInfo::default();
        get_proc_info(12196, &mut process_info).unwrap();
    }
}
