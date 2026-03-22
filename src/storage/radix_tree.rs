pub(super) struct RadixTree<T> {
    root: Node<T>,
}
impl<T> RadixTree<T> {
    pub fn new() -> Self {
        RadixTree {
            root: Node {
                children: Vec::new(),
                value: None,
            },
        }
    }

    pub fn search<'a>(&self, key: &'a str) -> Option<(&T, &'a str)> {
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
            ""
        };
        current_node.value.as_ref().map(|v| (v, matched))
    }
}

pub struct Node<T> {
    children: Vec<(String, Node<T>)>,
    value: Option<T>,
}
pub struct RadixTreeBuilder<T> {
    root: Node<T>,
}

impl<T> RadixTreeBuilder<T> {
    pub fn new() -> Self {
        RadixTreeBuilder {
            root: Node {
                children: Vec::new(),
                value: None,
            },
        }
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
    pub fn build(self) -> RadixTree<T> {
        RadixTree { root: self.root }
    }
}
