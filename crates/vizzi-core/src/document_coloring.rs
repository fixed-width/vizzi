//! On-demand styling of captured configuration and documentation files.

use crate::{
    coloring::{Coloring, INDEX_BYTES, Style},
    model::{CapturedSource, ContextId, SourceEncoding, SourceReadError},
    policy::DocumentFormat,
};
use jsonc_parser::{Scanner, ScannerOptions, errors::ParseErrorKind, tokens::Token};
use pulldown_cmark::{Event as MarkdownEvent, Options, Parser, Tag};
use std::sync::OnceLock;
use toml_parser::{
    Source,
    decoder::ScalarKind,
    parser::{Event, EventKind, RecursionGuard, parse_document},
};

const MAX_COLORING_INPUT: usize = 1024 * 1024;
const MAX_TOML_DEPTH: u32 = 80;

#[derive(Debug)]
pub(crate) struct DocumentColoring {
    format: DocumentFormat,
    index: OnceLock<Coloring>,
}

impl DocumentColoring {
    pub(crate) fn fingerprint() -> &'static ContextId {
        static FINGERPRINT: OnceLock<ContextId> = OnceLock::new();
        FINGERPRINT.get_or_init(|| {
            ContextId::derive(&[
                b"document-coloring-v1",
                include_bytes!("document_coloring.rs"),
                include_bytes!("coloring.rs"),
                include_bytes!("../../../Cargo.lock"),
            ])
        })
    }

    pub(crate) fn for_path(path: &str) -> Option<Self> {
        Some(Self {
            format: DocumentFormat::from_path(path)?,
            index: OnceLock::new(),
        })
    }

    pub(crate) fn index(&self, source: &CapturedSource) -> Option<&Coloring> {
        if let Some(index) = self.index.get() {
            return Some(index);
        }
        if source.encoding() != SourceEncoding::Utf8 {
            return None;
        }
        match source.read_text(MAX_COLORING_INPUT) {
            Ok(Some(text)) => Some(self.index.get_or_init(|| match self.format {
                DocumentFormat::Toml => toml_coloring(&text),
                DocumentFormat::Markdown => markdown_coloring(&text),
                DocumentFormat::Json => json_coloring(&text),
            })),
            Err(SourceReadError::Capacity) => Some(self.index.get_or_init(Coloring::limited)),
            _ => None,
        }
    }
}

fn toml_coloring(text: &str) -> Coloring {
    let source = Source::new(text);
    let tokens = source.lex().into_vec();
    let mut spans = Vec::new();
    let mut in_table_header = false;
    let mut depth = 0;
    let mut depth_exhausted = false;
    let mut receive = |event: Event| {
        use EventKind::*;
        let style = match event.kind() {
            StdTableOpen | ArrayTableOpen => {
                in_table_header = true;
                Style::Type
            }
            StdTableClose | ArrayTableClose => {
                in_table_header = false;
                Style::Type
            }
            SimpleKey | KeySep if in_table_header => Style::Type,
            SimpleKey | KeySep => Style::Attribute,
            ArrayOpen | InlineTableOpen => {
                depth += 1;
                // The guard forwards each open before deciding whether to enter it.
                depth_exhausted |= depth > i64::from(MAX_TOML_DEPTH) && !event.span().is_empty();
                return;
            }
            ArrayClose | InlineTableClose => {
                depth -= 1;
                return;
            }
            Comment => Style::Comment,
            Scalar if event.encoding().is_some() => Style::String,
            Scalar => {
                let Some(raw) = source.get(event) else { return };
                match raw.decode_scalar(&mut (), &mut ()) {
                    ScalarKind::Boolean(_) | ScalarKind::DateTime => Style::Constant,
                    ScalarKind::Float | ScalarKind::Integer(_) => Style::Number,
                    ScalarKind::String => return,
                }
            }
            Newline => {
                in_table_header = false;
                return;
            }
            _ => return,
        };
        let span = event.span();
        spans.push((span.start()..span.end(), style));
    };
    let mut guard = RecursionGuard::new(&mut receive, MAX_TOML_DEPTH);
    parse_document(&tokens, &mut guard, &mut ());
    if depth_exhausted {
        Coloring::limited()
    } else {
        Coloring::from_spans(spans, INDEX_BYTES)
    }
}

fn markdown_coloring(text: &str) -> Coloring {
    let options =
        Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS;
    let spans = Parser::new_ext(text, options)
        .into_offset_iter()
        .filter_map(|(event, mut range)| {
            let style = match event {
                MarkdownEvent::Start(Tag::Heading { .. }) | MarkdownEvent::Rule => Style::Keyword,
                MarkdownEvent::Start(Tag::Emphasis | Tag::Strong | Tag::Strikethrough)
                | MarkdownEvent::TaskListMarker(_) => Style::Attribute,
                MarkdownEvent::Start(Tag::Link { .. } | Tag::Image { .. }) => Style::Function,
                MarkdownEvent::Start(Tag::CodeBlock(_)) | MarkdownEvent::Code(_) => Style::String,
                MarkdownEvent::Start(Tag::HtmlBlock) | MarkdownEvent::InlineHtml(_) => {
                    let body = text.get(range.clone())?.strip_prefix("<!--")?;
                    if let Some(end) = body.find("-->") {
                        range.end = range.start + "<!--".len() + end + "-->".len();
                    }
                    Style::Comment
                }
                _ => return None,
            };
            Some((range, style))
        });
    Coloring::from_spans(spans, INDEX_BYTES)
}

fn json_coloring(text: &str) -> Coloring {
    let content = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut offset = text.len() - content.len();
    let options = ScannerOptions::default();
    let mut scanner = Scanner::new(content, &options);
    let tokens = std::iter::from_fn(|| match scanner.scan() {
        Ok(Some(token)) => {
            let range = offset + scanner.token_start()..offset + scanner.token_end();
            Some((range, token))
        }
        Err(error) if matches!(error.kind(), ParseErrorKind::String(_)) => {
            let start = offset + scanner.token_start();
            let end = start + json_string_len(&text[start..])?;
            scanner = Scanner::new(&text[end..], &options);
            offset = end;
            Some((start..end, Token::String(text[start + 1..end - 1].into())))
        }
        _ => None,
    });
    let spans = tokens.filter_map(|(range, token)| {
        let style = match token {
            Token::String(_) if json_colon_follows(&text[range.end..], &options) => {
                Style::Attribute
            }
            Token::String(_) => Style::String,
            Token::Number(_) => Style::Number,
            Token::Boolean(_) | Token::Null => Style::Constant,
            Token::CommentLine(_) | Token::CommentBlock(_) => Style::Comment,
            _ => return None,
        };
        Some((range, style))
    });
    Coloring::from_spans(spans, INDEX_BYTES)
}

fn json_string_len(text: &str) -> Option<usize> {
    let mut bytes = text.bytes().enumerate();
    let (_, quote) = bytes.next()?;
    while let Some((index, byte)) = bytes.next() {
        match byte {
            b'\\' => {
                bytes.next();
            }
            b'\r' | b'\n' => return None,
            _ if byte == quote => return Some(index + 1),
            _ => {}
        }
    }
    None
}

fn json_colon_follows(text: &str, options: &ScannerOptions) -> bool {
    let mut scanner = Scanner::new(text, options);
    while let Ok(Some(token)) = scanner.scan() {
        match token {
            Token::CommentLine(_) | Token::CommentBlock(_) => continue,
            _ => return matches!(token, Token::Colon),
        }
    }
    false
}
