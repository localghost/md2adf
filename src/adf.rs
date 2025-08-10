#![allow(dead_code)]

use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
struct Paragraph {
    #[serde(rename = "type")]
    block_type: &'static str,
    content: Vec<AdfNode>,
}

impl Paragraph {
    fn new() -> Self {
        Self {
            block_type: "paragraph",
            content: vec![],
        }
    }
}

#[derive(Debug, Serialize)]
struct CodeBlock {
    #[serde(rename = "type")]
    block_type: &'static str,
    content: Vec<AdfNode>,
}

impl CodeBlock {
    fn new() -> Self {
        Self {
            block_type: "codeBlock",
            content: vec![],
        }
    }
}

#[derive(Debug, Serialize)]
struct Text {
    #[serde(rename = "type")]
    block_type: &'static str,
    text: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    marks: Vec<Mark>,
}

impl Text {
    fn new(text: &str) -> Self {
        Self {
            block_type: "text",
            text: text.to_string(),
            marks: vec![],
        }
    }

    fn add_mark(&mut self, mark: Mark) {
        self.marks.push(mark);
    }
}

#[derive(Debug, Serialize)]
struct Mark {
    #[serde(rename = "type")]
    block_type: &'static str,
    attrs: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum AdfNode {
    Paragraph(Paragraph),
    CodeBlock(CodeBlock),
    Text(Text),
    Mark(Mark),
}

struct ParagraphBuilder<'a> {
    paragraph: &'a mut Paragraph,
}

impl<'a> ParagraphBuilder<'a> {
    fn new(paragraph: &'a mut Paragraph) -> Self {
        Self { paragraph }
    }

    fn text(self, text: &str) -> Self {
        self.paragraph.content.push(AdfNode::Text(Text::new(text)));
        self
    }

    fn link(self, text: &str, url: &str) -> Self {
        let mut text = Text::new(text);
        text.add_mark(Mark {
            block_type: "link",
            attrs: HashMap::from([("href".to_string(), url.to_string())]),
        });
        self.paragraph.content.push(AdfNode::Text(text));
        self
    }
}

struct CodeBlockBuilder<'a> {
    code_block: &'a mut CodeBlock,
}

impl<'a> CodeBlockBuilder<'a> {
    fn new(code_block: &'a mut CodeBlock) -> Self {
        Self { code_block }
    }

    fn text(self, text: &str) -> Self {
        self.code_block.content.push(AdfNode::Text(Text::new(text)));
        self
    }
}

#[derive(Debug, Serialize)]
struct Document {
    content: Vec<AdfNode>,
    #[serde(rename = "type")]
    block_type: &'static str,
    version: u32,
}

#[derive(Debug)]
struct DocumentBuilder {
    content: Vec<AdfNode>,
}

impl DocumentBuilder {
    fn new() -> Self {
        Self { content: vec![] }
    }

    fn paragraph(&mut self) -> ParagraphBuilder {
        self.content.push(AdfNode::Paragraph(Paragraph::new()));
        if let AdfNode::Paragraph(p) = self.content.last_mut().unwrap() {
            ParagraphBuilder::new(p)
        } else {
            unreachable!()
        }
    }

    fn code_block(&mut self) -> CodeBlockBuilder {
        self.content.push(AdfNode::CodeBlock(CodeBlock::new()));
        if let AdfNode::CodeBlock(cb) = self.content.last_mut().unwrap() {
            CodeBlockBuilder::new(cb)
        } else {
            unreachable!()
        }
    }

    fn build(self) -> anyhow::Result<Document> {
        Ok(Document {
            content: self.content,
            block_type: "doc",
            version: 1,
        })
    }
}

pub fn from_markdown(md: &str) -> anyhow::Result<String> {
    let md = markdown::to_mdast(md, &markdown::ParseOptions::default()).unwrap();
    let mut document_builder = DocumentBuilder::new();
    for node in md.children().unwrap().iter() {
        match node {
            markdown::mdast::Node::Paragraph(p) => {
                let mut paragraph = document_builder.paragraph();
                for node in p.children.iter() {
                    paragraph = match node {
                        markdown::mdast::Node::Text(t) => paragraph.text(&t.value),
                        markdown::mdast::Node::Link(l) => {
                            // Use url as the link text if no text is provided
                            let text = l.children.first().map_or(&l.url, |v| {
                                if let markdown::mdast::Node::Text(t) = v {
                                    &t.value
                                } else {
                                    &l.url
                                }
                            });
                            paragraph.link(text, &l.url)
                        }
                        node => anyhow::bail!(
                            "Only text nodes are supported for paragraph node, found {:?}",
                            node
                        ),
                    };
                }
            }
            markdown::mdast::Node::Code(c) => {
                document_builder.code_block().text(&c.value);
            }
            node => anyhow::bail!(
                "Only paragraph and code nodes are supported, found {:?}",
                node
            ),
        }
    }
    Ok(serde_json::to_string(&document_builder.build()?)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_text_paragraph() {
        let expected = r#"{"version": 1, "type": "doc", "content": [{"type": "paragraph", "content": [{"type": "text", "text": "this is some paragraph"}]}]}"#;
        let actual = from_markdown("this is some paragraph").unwrap();
        assert_eq!(
            expected.parse::<serde_json::Value>().unwrap(),
            actual.parse::<serde_json::Value>().unwrap()
        );
    }

    #[test]
    fn paragraph_with_links() {
        let expected = r#"{"version": 1, "type": "doc", "content": [
            {"type": "paragraph", "content": [
                {"type": "text", "text": "alamakota", "marks": [
                    {"type": "link", "attrs": {"href": "http://duckduck.go"}}
                ]},
                {"type": "text", "text": " "},
                {"type": "text", "text": "http://google.com", "marks": [
                    {"type": "link", "attrs": {"href": "http://google.com"}}
                ]},
                {"type": "text", "text": " this is some paragraph"}
            ]}
        ]}"#;
        let actual = from_markdown(
            "[alamakota](http://duckduck.go) <http://google.com> this is some paragraph",
        )
        .unwrap();
        assert_eq!(
            expected.parse::<serde_json::Value>().unwrap(),
            actual.parse::<serde_json::Value>().unwrap()
        );
    }

    #[test]
    fn code_block() {
        let expected = r#"{"version": 1, "type": "doc", "content": [{"type": "codeBlock", "content": [{"type": "text", "text": "a = 42"}]}]}"#;
        let actual = from_markdown("```\na = 42\n```").unwrap();
        assert_eq!(
            expected.parse::<serde_json::Value>().unwrap(),
            actual.parse::<serde_json::Value>().unwrap()
        );
    }
}
