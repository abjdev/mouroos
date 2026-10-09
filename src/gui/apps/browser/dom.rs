use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Debug)]
pub struct DomNode {
    pub id: usize,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub tag: String, // lowercase, e.g. "div", "button", "#text"
    pub text: String, // only non-empty if text node or cached
    pub attrs: BTreeMap<String, String>,
    pub id_attr: Option<String>,
    pub classes: Vec<String>,
    pub inline_style: String,
    pub onclick: Option<String>,
}

impl DomNode {
    pub fn new(id: usize, tag: &str) -> Self {
        DomNode {
            id,
            parent: None,
            children: Vec::new(),
            tag: String::from(tag).to_lowercase(),
            text: String::new(),
            attrs: BTreeMap::new(),
            id_attr: None,
            classes: Vec::new(),
            inline_style: String::new(),
            onclick: None,
        }
    }

    pub fn new_text(id: usize, text: &str) -> Self {
        let mut node = DomNode::new(id, "#text");
        node.text = String::from(text);
        node
    }

    pub fn is_text(&self) -> bool {
        self.tag == "#text"
    }

    pub fn has_class(&self, class_name: &str) -> bool {
        self.classes.iter().any(|c| c == class_name)
    }
}

#[derive(Clone, Debug)]
pub struct DomTree {
    pub nodes: Vec<DomNode>,
    pub root: usize,
}

impl DomTree {
    pub fn new() -> Self {
        let root_node = DomNode::new(0, "#document");
        DomTree {
            nodes: alloc::vec![root_node],
            root: 0,
        }
    }

    pub fn create_element(&mut self, tag: &str) -> usize {
        let id = self.nodes.len();
        self.nodes.push(DomNode::new(id, tag));
        id
    }

    pub fn create_text_node(&mut self, text: &str) -> usize {
        let id = self.nodes.len();
        self.nodes.push(DomNode::new_text(id, text));
        id
    }

    pub fn append_child(&mut self, parent: usize, child: usize) {
        if parent < self.nodes.len() && child < self.nodes.len() {
            self.nodes[child].parent = Some(parent);
            self.nodes[parent].children.push(child);
        }
    }

    pub fn get_node(&self, id: usize) -> Option<&DomNode> {
        self.nodes.get(id)
    }

    pub fn get_node_mut(&mut self, id: usize) -> Option<&mut DomNode> {
        self.nodes.get_mut(id)
    }

    pub fn set_attribute(&mut self, node_id: usize, key: &str, val: &str) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            let key_lower = key.to_lowercase();
            node.attrs.insert(key_lower.clone(), String::from(val));
            match key_lower.as_str() {
                "id" => node.id_attr = Some(String::from(val)),
                "class" => {
                    node.classes = val.split_whitespace().map(String::from).collect();
                }
                "style" => {
                    node.inline_style = String::from(val);
                }
                "onclick" => {
                    node.onclick = Some(String::from(val));
                }
                _ => {}
            }
        }
    }

    pub fn get_attribute(&self, node_id: usize, key: &str) -> Option<&str> {
        self.nodes.get(node_id)?.attrs.get(&key.to_lowercase()).map(|s| s.as_str())
    }

    pub fn get_element_by_id(&self, target_id: &str) -> Option<usize> {
        for node in &self.nodes {
            if let Some(ref id) = node.id_attr {
                if id == target_id {
                    return Some(node.id);
                }
            }
        }
        None
    }

    pub fn get_elements_by_tag_name(&self, tag: &str) -> Vec<usize> {
        let tag_lower = tag.to_lowercase();
        self.nodes
            .iter()
            .filter(|n| n.tag == tag_lower)
            .map(|n| n.id)
            .collect()
    }

    pub fn get_elements_by_class_name(&self, class_name: &str) -> Vec<usize> {
        self.nodes
            .iter()
            .filter(|n| n.has_class(class_name))
            .map(|n| n.id)
            .collect()
    }

    pub fn get_inner_text(&self, node_id: usize) -> String {
        let mut out = String::new();
        self.collect_text(node_id, &mut out);
        out
    }

    fn collect_text(&self, node_id: usize, out: &mut String) {
        if let Some(node) = self.nodes.get(node_id) {
            if node.is_text() {
                out.push_str(&node.text);
            } else {
                for &child_id in &node.children {
                    self.collect_text(child_id, out);
                }
            }
        }
    }

    pub fn set_inner_text(&mut self, node_id: usize, text: &str) {
        if node_id >= self.nodes.len() {
            return;
        }

        if self.nodes[node_id].is_text() {
            self.nodes[node_id].text = String::from(text);
            return;
        }

        // Replace all children with a single text node
        self.nodes[node_id].children.clear();
        let text_child = self.create_text_node(text);
        self.append_child(node_id, text_child);
    }
}

#[test_case]
fn test_dom_tree_operations() {
    let mut tree = DomTree::new();
    let div = tree.create_element("div");
    tree.append_child(tree.root, div);
    tree.set_attribute(div, "id", "container");
    tree.set_attribute(div, "class", "box active");

    let p = tree.create_element("p");
    tree.append_child(div, p);
    tree.set_inner_text(p, "Hello World");

    assert_eq!(tree.get_element_by_id("container"), Some(div));
    assert_eq!(tree.get_elements_by_class_name("box"), alloc::vec![div]);
    assert_eq!(tree.get_inner_text(div), "Hello World");

    tree.set_inner_text(p, "Updated Text");
    assert_eq!(tree.get_inner_text(div), "Updated Text");
}

