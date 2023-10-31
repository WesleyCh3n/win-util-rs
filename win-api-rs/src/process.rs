use std::collections::VecDeque;

use std::cell::RefCell;
use std::rc::Rc;

use windows::core::Result;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32,
    TH32CS_SNAPPROCESS,
};

struct PidTree {
    pid: u32,
    children: Vec<Rc<RefCell<PidTree>>>,
}

impl PidTree {
    fn new(pid: u32) -> PidTree {
        PidTree {
            pid,
            children: Vec::new(),
        }
    }
    fn find_child(&mut self, pid: u32) -> Option<&mut PidTree> {
        let mut queue = VecDeque::new();
        queue.push_back(self.pid);
        while let Some(pid) = queue.pop_front() {
            //
        }
        None
    }
}

pub fn find_pidtree(pid: u32) -> Result<Vec<u32>> {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).unwrap();
        let mut entry = PROCESSENTRY32 {
            dwSize: std::mem::size_of::<PROCESSENTRY32>() as u32,
            ..Default::default()
        };
        Process32First(snapshot, &mut entry as *mut PROCESSENTRY32)?;
        let mut pids = vec![pid];
        let mut tree = PidTree::new(pid);
        loop {
            for i in 0..pids.len() {
                if pids[i] == entry.th32ParentProcessID {
                    pids.push(entry.th32ProcessID);
                }
            }
            if Process32Next(snapshot, &mut entry as *mut PROCESSENTRY32)
                .is_err()
            {
                break;
            }
        }
        Ok(pids)
    }
}
