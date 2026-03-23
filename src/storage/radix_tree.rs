use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub(super) struct RadixTree<T: Clone> {
    pub root: Node<T>,
}
impl<T: Clone> RadixTree<T> {
    pub fn new() -> Self {
        RadixTree {
            root: Node {
                children: Vec::new(),
                value: None,
            },
        }
    }

    pub fn search<'a>(&self, key: &'a str) -> Option<(&T, &'a str)> {
        // dbg!(key);
        let mut current_node = &self.root;
        let key = key.trim_matches('/');
        let mut find_idx = 0;
        'search_loop: for key_part in key.split('/') {
            for (prefix, child) in &current_node.children {
                if key_part == prefix {
                    current_node = child;
                    find_idx += prefix.len() + 1;
                    continue 'search_loop;
                }
            }
            break;
        }
        let matched = if find_idx > 0 {
            &key[find_idx - 1..]
        } else {
            return None;
        };
        // dbg!(matched);
        current_node.value.as_ref().map(|v| (v, matched))
    }
    pub fn insert(&mut self, key: &str, value: T) {
        let key = key.trim_matches('/');
        let mut current_node = &mut self.root;

        for key_part in key.split('/') {
            let mut found = false;
            let mut index = 0;
            for (i, (prefix, _)) in current_node.children.iter().enumerate() {
                if key_part == prefix {
                    index = i;
                    found = true;
                    break;
                }
            }

            if found {
                current_node = &mut current_node.children[index].1;
            } else {
                let new_node = Node {
                    children: Vec::new(),
                    value: None,
                };
                current_node.children.push((key_part.to_string(), new_node));
                current_node = &mut current_node.children.last_mut().unwrap().1;
            }
        }

        current_node.value = Some(value);
    }
    pub fn search_children<'a>(&self, key: &'a str) -> &Vec<(String, Node<T>)> {
        let key = key.trim_matches('/');
        let mut current_node = &self.root;
        'search_loop: for key_part in key.split('/') {
            for (prefix, child) in &current_node.children {
                if key_part == prefix {
                    current_node = child;
                    continue 'search_loop;
                }
            }
            break;
        }

        &current_node.children
    }

    pub fn remove(&mut self, key: &str) -> Option<T> {
        let key = key.trim_matches('/');
        let mut current_node = &mut self.root;
        let mut path: Vec<(&mut Node<T>, usize)> = Vec::new();

        for key_part in key.split('/') {
            let mut found = false;
            let mut child_index = None;
            for (i, (prefix, _)) in current_node.children.iter().enumerate() {
                if key_part == prefix {
                    child_index = Some(i);
                    found = true;
                    break;
                }
            }
            if !found {
                return None;
            }
            if let Some(idx) = child_index {
                path.push((unsafe { &mut *(current_node as *mut _) }, idx));
                current_node = &mut current_node.children[idx].1;
            }
        }

        let value = current_node.value.take();

        // Clean up empty nodes
        while let Some((parent, idx)) = path.pop() {
            if current_node.children.is_empty() && current_node.value.is_none() {
                parent.children.remove(idx);
                current_node = parent;
            } else {
                break;
            }
        }

        value
    }

    pub fn clear(&mut self) {
        self.root.children.clear();
    }
    pub fn iter(&self) -> RadixTreeIterable<'_, T> {
        RadixTreeIterable::new(self)
    }
    pub fn iter_path(&self) -> RadixTreePathIterable<'_, T> {
        RadixTreePathIterable::new(self)
    }
}
#[derive(Debug, Clone)]
pub struct Node<T> {
    pub children: Vec<(String, Node<T>)>,
    pub value: Option<T>,
}
pub struct RadixTreeIterable<'a, T: Clone> {
    stack: VecDeque<&'a Node<T>>,
}
impl<'a, T: Clone> RadixTreeIterable<'a, T> {
    pub fn new(tree: &'a RadixTree<T>) -> Self {
        let mut stack = VecDeque::new();
        stack.push_back(&tree.root);
        RadixTreeIterable { stack }
    }
}
impl<'a, T: Clone> Iterator for RadixTreeIterable<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(node) = self.stack.pop_front() {
            self.stack
                .extend(node.children.iter().map(|(_, child)| child));
            if let Some(value) = &node.value {
                return Some(value);
            }
        }
        None
    }
}
pub struct RadixTreePathIterable<'a, T: Clone> {
    stack: VecDeque<(&'a Node<T>, String)>,
}

impl<'a, T: Clone> RadixTreePathIterable<'a, T> {
    pub fn new(tree: &'a RadixTree<T>) -> Self {
        let mut stack = VecDeque::new();
        stack.push_back((&tree.root, String::new()));
        RadixTreePathIterable { stack }
    }
}

impl<'a, T: Clone> Iterator for RadixTreePathIterable<'a, T> {
    type Item = (String, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((node, path)) = self.stack.pop_front() {
            for (prefix, child) in &node.children {
                let mut new_path = path.clone();
                new_path.push('/');
                new_path.push_str(prefix);
                self.stack.push_back((child, new_path));
            }
            if let Some(value) = &node.value {
                return Some((path, value));
            }
        }
        None
    }
}
