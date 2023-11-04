use std::collections::VecDeque;

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
    dfs(root, "".to_string(), false);
}

fn dfs(root: Rc<RefCell<PidNode>>, mut prefix: String, have_siblings: bool) {
    println!("{}{} {}", prefix, root.borrow().pid, root.borrow().name);
    // if I have siblings and i have child
    if have_siblings && !root.borrow().children.is_empty() {
        prefix.replace_range(prefix.len() - 4.., " |  ");
    }
    // if I don't have siblings and i have child
    if !have_siblings && !root.borrow().children.is_empty() && prefix.len() > 4
    {
        prefix.replace_range(prefix.len() - 4.., "    ");
    }
    // if I have child?
    if !root.borrow().children.is_empty() {
        prefix.push_str(" \\_ ");
    }
    for (i, child) in root.borrow().children.iter().enumerate() {
        dfs(
            child.clone(),
            prefix.clone(),
            i != root.borrow().children.len() - 1,
        );
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

pub fn find_pidtree(pid: u32) -> Option<Rc<RefCell<PidNode>>> {
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
        let tree = find_pidtree(7980);
        // println!("{:?}", tree);
        if let Some(tree) = tree {
            print(tree);
        }
    }
}
