use std::collections::{HashSet, VecDeque};

use std::cell::RefCell;
use std::rc::Rc;

use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32,
    TH32CS_SNAPPROCESS,
};

#[derive(Debug, Default)]
pub struct PidNode {
    pid: u32,
    name: String,
    children: Vec<Rc<RefCell<PidNode>>>,
}

impl PidNode {
    fn new(pid: u32, name: String) -> PidNode {
        PidNode {
            pid,
            name,
            children: Vec::new(),
        }
    }
}

pub fn print(root: Rc<RefCell<PidNode>>) {
    dfs(root, Vec::new(), false);
}

fn dfs(root: Rc<RefCell<PidNode>>, mut prefix: Vec<char>, have_siblings: bool) {
    println!(
        "{: <6} {} {}",
        root.borrow().pid,
        prefix.iter().collect::<String>(),
        root.borrow().name
    );
    let children = &root.borrow().children;
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
    for (i, child) in children.iter().enumerate() {
        let last_child = i == children.len() - 1;
        if last_child {
            prefix.truncate(prefix.len() - 4);
            prefix.append(&mut vec![' ', '╚', '═', ' ']);
        }
        dfs(child.clone(), prefix.clone(), !last_child);
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
                tree = Some(Rc::new(RefCell::new(PidNode::new(
                    pid,
                    str_from_u8(&entry.szExeFile),
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
                    node.borrow_mut().children.push(Rc::new(RefCell::new(
                        PidNode::new(
                            entry.th32ProcessID,
                            str_from_u8(&entry.szExeFile),
                        ),
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

fn str_from_u8(bytes: &[u8]) -> String {
    std::str::from_utf8(
        &bytes[0..bytes
            .iter()
            .position(|&c| c == b'\0')
            .unwrap_or(bytes.len())],
    )
    .unwrap()
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_test() {
        let tree = find_by_pid(7980);
        if let Some(tree) = tree {
            println!("{: <6}  Process Name", "PID");
            print(tree);
        }
    }

    #[test]
    fn list_test() {
        let processes = list_all();
        println!("{: <6}  Process Name", "PID");
        for process in processes {
            print(process);
        }
    }
}
