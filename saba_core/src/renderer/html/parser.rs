use crate::renderer::dom::node::{Element, ElementKind, Node, NodeKind, Window};
use crate::renderer::html::attribute::Attribute;
use crate::renderer::html::token::{HtmlToken, HtmlTokenizer};
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::str::FromStr;

#[derive(Debug, Clone)]
pub struct HtmlParser {
    window: Rc<RefCell<Window>>,
    mode: InsertionMode,
    original_insertion_mode: InsertionMode,
    stack_of_open_elements: Vec<Rc<RefCell<Node>>>,
    t: HtmlTokenizer,
}

impl HtmlParser {
    pub fn new(t: HtmlTokenizer) -> Self {
        Self {
            window: Rc::new(RefCell::new(Window::new())),
            mode: InsertionMode::Initial,
            original_insertion_mode: InsertionMode::Initial,
            stack_of_open_elements: Vec::new(),
            t,
        }
    }

    pub fn construct_tree(&mut self) -> Rc<RefCell<Window>> {
        let mut token = self.t.next();

        while token.is_some() {
            match self.mode {
                InsertionMode::Initial => {
                    // DOCTYPEをサポートしていないためそれは文字トークンとして扱われる
                    // 本実装ではそれを無視することにしている
                    if let Some(HtmlToken::Char(_)) = token {
                        token = self.t.next();
                        continue;
                    }

                    self.mode = InsertionMode::BeforeHtml;
                    continue;
                }
                InsertionMode::BeforeHtml => {
                    match token {
                        Some(HtmlToken::Char(c)) => {
                            if c == ' ' || c == '\n' {
                                token = self.t.next();
                                continue;
                            }
                        }
                        Some(HtmlToken::StartTag { ref tag, ref attributes, .. }) => {
                            if tag == "html" {
                                self.insert_element(tag, attributes.to_vec());
                                self.mode = InsertionMode::BeforeHead;
                                token = self.t.next();
                                continue;
                            }
                        }
                        Some(HtmlToken::Eof) | None => {
                            return self.window.clone()
                        }
                        _ => {}
                    }

                    self.insert_element("html", Vec::new());
                    self.mode = InsertionMode::BeforeHead;
                    continue;
                }
                InsertionMode::BeforeHead => {
                    match token {
                        Some(HtmlToken::Char(c)) => {
                            if c == ' ' || c == '\n' {
                                token = self.t.next();
                                continue;
                            }
                        }
                        Some(HtmlToken::StartTag { ref tag, ref attributes, .. }) => {
                            if tag == "head" {
                                self.insert_element(tag, attributes.to_vec());
                                self.mode = InsertionMode::InHead;
                                token = self.t.next();
                                continue;
                            }
                        }
                        Some(HtmlToken::Eof) | None => {
                            return self.window.clone();
                        }
                        _ => {}
                    }

                    self.insert_element("head", Vec::new());
                    self.mode = InsertionMode::InHead;
                    continue;
                }
                InsertionMode::InHead => {
                    match token {
                        Some(HtmlToken::Char(c)) => {
                            if c == ' ' || c == '\n' {
                                self.insert_char(c);
                                token = self.t.next();
                                continue;
                            }
                        }
                        Some(HtmlToken::StartTag { ref tag, ref attributes, .. }) => {
                            if tag == "style" || tag == "script" {
                                self.insert_element(tag, attributes.to_vec());
                                self.original_insertion_mode = self.mode;
                                self.mode = InsertionMode::Text;
                                token = self.t.next();
                                continue;
                            }

                            // このブラウザは仕様をすべて実装しているわけではないため、<head>が省略されているHTMLを扱うために必要
                            // これがないと<head>が省略されているHTMLで無限ループが発生する
                            if tag == "body" {
                                self.pop_until(ElementKind::Head);
                                self.mode = InsertionMode::AfterHead;
                                continue;
                            }

                            if let Ok(_) = ElementKind::from_str(tag) {
                                self.pop_until(ElementKind::Head);
                                self.mode = InsertionMode::AfterHead;
                                continue;
                            }
                        }
                        Some(HtmlToken::EndTag { ref tag }) => {
                            if tag == "head" {
                                self.mode = InsertionMode::AfterHead;
                                token = self.t.next();
                                self.pop_until(ElementKind::Head);
                                continue;
                            }
                        }
                        Some(HtmlToken::Eof) | None => {
                            return self.window.clone();
                        }
                    }

                    token = self.t.next();
                    continue;
                }
                InsertionMode::AfterHead => {
                    match token {
                        Some(HtmlToken::Char(c)) => {
                            if c == ' ' || c == '\n' {
                                self.insert_char(c);
                                token = self.t.next();
                                continue;
                            }
                        }
                        Some(HtmlToken::StartTag {
                                 ref tag,
                                 ref attributes,
                                 ..
                             }) => {
                            if tag == "body" {
                                self.insert_element(tag, attributes.to_vec());
                                token = self.t.next();
                                self.mode = InsertionMode::InBody;
                                continue;
                            }
                        }
                        Some(HtmlToken::Eof) | None => {
                            return self.window.clone();
                        }
                        _ => {}
                    }

                    self.insert_element("body", Vec::new());
                    self.mode = InsertionMode::InBody;
                    continue;
                }
                InsertionMode::InBody => {
                    match token {
                        Some(HtmlToken::StartTag { ref tag, ref attributes, .. }) => {
                            match tag.as_str() {
                                "p" | "h1" | "h2" | "a" => {
                                    self.insert_element(tag, attributes.to_vec());
                                    token = self.t.next();
                                    continue;
                                }
                                _ => {
                                    token = self.t.next();
                                }
                            }
                        }
                        Some(HtmlToken::EndTag { ref tag }) => {
                            match tag.as_str() {
                                "body" => {
                                    self.mode = InsertionMode::AfterBody;
                                    token = self.t.next();
                                    if !self.contain_in_stack(ElementKind::Body) {
                                        // Failed to parse. Skip the token.
                                        continue;
                                    }
                                    self.pop_until(ElementKind::Body);
                                    continue;
                                }
                                "html" => {
                                    if self.pop_current_node(ElementKind::Body) {
                                        self.mode = InsertionMode::AfterBody;
                                        assert!(self.pop_current_node(ElementKind::Html))
                                    } else {
                                        token = self.t.next();
                                    }
                                    continue;
                                }
                                "p" | "h1" | "h2" | "a" => {
                                    let element_kind = ElementKind::from_str(tag).expect("Failed to convert string to ElementKind");
                                    token = self.t.next();
                                    self.pop_until(element_kind);
                                    continue;
                                }
                                _ => {
                                    token = self.t.next();
                                }
                            }
                        }
                        Some(HtmlToken::Char(c)) => {
                            self.insert_char(c);
                            token = self.t.next();
                            continue;
                        }
                        Some(HtmlToken::Eof) | None => {
                            return self.window.clone();
                        }
                    }
                }
                InsertionMode::Text => {
                    match token {
                        Some(HtmlToken::Eof) | None => {
                            return self.window.clone();
                        }
                        Some(HtmlToken::EndTag { ref tag }) => {
                            match tag.as_str() {
                                "style" => {
                                    self.pop_until(ElementKind::Style);
                                    self.mode = self.original_insertion_mode;
                                    token = self.t.next();
                                    continue;
                                }
                                "script" => {
                                    self.pop_until(ElementKind::Script);
                                    self.mode = self.original_insertion_mode;
                                    token = self.t.next();
                                    continue;
                                }
                                _ => {}
                            }
                        }
                        Some(HtmlToken::Char(c)) => {
                            self.insert_char(c);
                            token = self.t.next();
                            continue;
                        }
                        _ => {}
                    }

                    self.mode = self.original_insertion_mode;
                }
                InsertionMode::AfterBody => {
                    match token {
                        Some(HtmlToken::Char(_)) => {
                            token = self.t.next();
                            continue;
                        }
                        Some(HtmlToken::EndTag { ref tag }) => {
                            if tag == "html" {
                                self.mode = InsertionMode::AfterAfterBody;
                                token = self.t.next();
                                continue;
                            }
                        }
                        Some(HtmlToken::Eof) | None => {
                            return self.window.clone();
                        }
                        _ => {}
                    }

                    self.mode = InsertionMode::InBody;
                }
                InsertionMode::AfterAfterBody => {
                    match token {
                        Some(HtmlToken::Char(_)) => {
                            token = self.t.next();
                            continue;
                        }
                        Some(HtmlToken::Eof) | None => {
                            return self.window.clone();
                        }
                        _ => {}
                    }

                    // Failed to parse
                    self.mode = InsertionMode::InBody;
                }
            }
        }

        self.window.clone()
    }

    fn insert_element(&mut self, tag: &str, attributes: Vec<Attribute>) {
        let window = self.window.borrow();
        let current = match self.stack_of_open_elements.last() {
            Some(n) => n.clone(),
            None => window.document(),
        };
        let node = Rc::new(RefCell::new(create_element_node(tag, attributes)));

        let mut current_borrowed = current.borrow_mut();
        match current_borrowed.first_child() {
            Some(ref first_child) => {
                let mut last_sibling = Rc::clone(first_child);
                loop {
                    let next = last_sibling.borrow_mut().next_sibling();
                    match next {
                        Some(ref n) => {
                            last_sibling = Rc::clone(n);
                        }
                        None => {
                            break;
                        }
                    }
                }

                last_sibling.borrow_mut().set_next_sibling(Some(node.clone()));
                node.borrow_mut().set_previous_sibling(Rc::downgrade(first_child))
            }
            None => {
                current_borrowed.set_first_child(Some(node.clone()));
            }
        }

        current_borrowed.set_last_child(Rc::downgrade(&node));
        node.borrow_mut().set_parent(Rc::downgrade(&current));
        self.stack_of_open_elements.push(node);
    }

    fn pop_current_node(&mut self, element_kind: ElementKind) -> bool {
        let current = match self.stack_of_open_elements.last() {
            Some(n) => n,
            None => return false,
        };

        if current.borrow().element_kind() == Some(element_kind) {
            self.stack_of_open_elements.pop();
            return true;
        }

        false
    }

    fn pop_until(&mut self, element_kind: ElementKind) {
        assert!(
            self.contain_in_stack(element_kind),
            "stack doesn't have an element {:?}",
            element_kind,
        );

        loop {
            let current = match self.stack_of_open_elements.pop() {
                Some(n) => n,
                None => return,
            };

            if current.borrow().element_kind() == Some(element_kind) {
                return;
            }
        }
    }

    fn contain_in_stack(&self, element_kind: ElementKind) -> bool {
        for i in 0..self.stack_of_open_elements.len() {
            if self.stack_of_open_elements[i].borrow().element_kind() == Some(element_kind) {
                return true;
            }
        }

        false
    }

    fn insert_char(&mut self, c: char) {
        let current = match self.stack_of_open_elements.last() {
            Some(n) => n.clone(),
            None => return,
        };

        if let NodeKind::Text(ref mut s) = current.borrow_mut().kind {
            s.push(c);
            return;
        }

        if c == ' ' || c == '\n' {
            return;
        }

        let node = Rc::new(RefCell::new(create_char_node(c)));

        let mut current_borrowed = current.borrow_mut();
        match current_borrowed.first_child() {
            Some(first_child) => {
                first_child.borrow_mut().set_next_sibling(Some(node.clone()));
                node.borrow_mut().set_previous_sibling(Rc::downgrade(&first_child));
            }
            None => {
                current_borrowed.set_first_child(Some(node.clone()));
            }
        }

        current_borrowed.set_last_child(Rc::downgrade(&node));
        node.borrow_mut().set_parent(Rc::downgrade(&current));
        self.stack_of_open_elements.push(node);
    }
}

fn create_element_node(tag: &str, attributes: Vec<Attribute>) -> Node {
    Node::new(NodeKind::Element(Element::new(tag, attributes)))
}

fn create_char_node(c: char) -> Node {
    Node::new(NodeKind::Text(String::from(c)))
}


#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum InsertionMode {
    Initial,
    BeforeHtml,
    BeforeHead,
    InHead,
    AfterHead,
    InBody,
    Text,
    AfterBody,
    AfterAfterBody,
}

#[cfg(test)]
mod tests {
    use crate::renderer::dom::node::{Element, Node, NodeKind};
    use crate::renderer::html::attribute::Attribute;
    use crate::renderer::html::parser::HtmlParser;
    use crate::renderer::html::token::HtmlTokenizer;
    use alloc::rc::Rc;
    use alloc::string::ToString;
    use alloc::vec;
    use alloc::vec::Vec;
    use core::cell::RefCell;

    #[test]
    fn test_empty() {
        let html = "".to_string();
        let t = HtmlTokenizer::new(html);
        let window = HtmlParser::new(t).construct_tree();
        let expected = Rc::new(RefCell::new(Node::new(NodeKind::Document)));

        assert_eq!(expected, window.borrow().document());
    }
    #[test]
    fn test_body() {
        let html = "<html><head></head><body></body></html>".to_string();
        let t = HtmlTokenizer::new(html);
        let window = HtmlParser::new(t).construct_tree();
        let document = window.borrow().document();
        assert_eq!(Rc::new(RefCell::new(Node::new(NodeKind::Document))), document);

        let html = document.borrow().first_child().expect("Failed to get a first child of document");
        assert_eq!(Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new("html", Vec::new()))))), html);

        let head = html.borrow().first_child().expect("Failed to get a first child of html");
        assert_eq!(Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new("head", Vec::new()))))), head);

        let body = head.borrow().next_sibling().expect("Failed to get a next sibling of head");
        assert_eq!(Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new("body", Vec::new()))))), body);
    }

    #[test]
    fn test_text() {
        let html = "<html><head></head><body>text</body></html>".to_string();
        let t = HtmlTokenizer::new(html);
        let window = HtmlParser::new(t).construct_tree();
        let document = window.borrow().document();
        assert_eq!(Rc::new(RefCell::new(Node::new(NodeKind::Document))), document);

        let html = document.borrow().first_child().expect("Failed to get a first child of document");
        assert_eq!(Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new("html", Vec::new()))))), html);

        let body = html.borrow().first_child().expect("Failed to get a first child of html").borrow().next_sibling().expect("Failed to get a next sibling of head");
        assert_eq!(Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new("body", Vec::new()))))), body);

        let text = body.borrow().first_child().expect("Failed to get a first child of body");
        assert_eq!(Rc::new(RefCell::new(Node::new(NodeKind::Text("text".to_string())))), text);
    }

    #[test]
    fn test_multiple_nodes() {
        let html = "<html><head></head><body><p><a foo=bar>text</a></p></body></html>".to_string();
        let t = HtmlTokenizer::new(html);
        let window = HtmlParser::new(t).construct_tree();
        let document = window.borrow().document();

        let body = document.borrow()
            .first_child()
            .expect("Failed to get a first child of document")
            .borrow()
            .first_child()
            .expect("Failed to get a first child of html")
            .borrow()
            .next_sibling()
            .expect("Failed to get a next sibling of head");
        assert_eq!(Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new("body", Vec::new()))))), body);

        let p = body.borrow().first_child().expect("Failed to get a first child of body");
        assert_eq!(Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new("p", Vec::new()))))), p);

        let mut attr = Attribute::new();
        "foo".chars().for_each(|c| attr.add_char(c, true));
        "bar".chars().for_each(|c| attr.add_char(c, false));
        let a = p.borrow().first_child().expect("Failed to get a first child of p");
        assert_eq!(Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new("a", vec![attr]))))), a);

        let text = a.borrow().first_child().expect("Failed to get a first child of a");
        assert_eq!(Rc::new(RefCell::new(Node::new(NodeKind::Text("text".to_string())))), text);
    }
}
