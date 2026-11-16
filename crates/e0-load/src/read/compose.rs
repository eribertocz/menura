//! Composes the parser events of the single document into a raw node tree (E0A-2.6), before any
//! tag or scalar is interpreted.

use granit_parser::{Event, Marker, Parser, ScalarStyle, ScanError, Span, Tag};

use super::error::{ReadError, ReadErrorCode};
use super::tree::Location;

/// The prefix of the standard YAML tags (`!!int` is `tag:yaml.org,2002:int`).
const STANDARD_TAG_PREFIX: &str = "tag:yaml.org,2002:";

/// A node as written, with its tag and anchor, and aliases not yet expanded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawNode {
    pub(crate) content: RawContent,
    pub(crate) tag: RawTag,
    /// The anchor identifier given by the parser; `0` when the node has no anchor.
    pub(crate) anchor: usize,
    /// Where the content starts.
    pub(crate) location: Location,
    /// Where the tag starts; the content location when the node has no tag.
    pub(crate) tag_location: Location,
}

/// The content of a raw node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RawContent {
    /// A scalar; `plain` is false when it is quoted or a block scalar.
    Scalar {
        text: String,
        plain: bool,
    },
    Sequence(Vec<RawNode>),
    Mapping(Vec<(RawNode, RawNode)>),
    /// An alias to the anchor with this identifier.
    Alias(usize),
}

/// The explicit tag of a raw node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RawTag {
    /// No tag: the node is resolved implicitly.
    Absent,
    /// The non-specific tag `!`.
    NonSpecific,
    /// A standard tag, by its suffix (`int` for `!!int`).
    Standard(String),
    /// Any other tag, as its full name.
    Other(String),
}

/// The properties shared by every node event.
struct Header {
    tag: RawTag,
    anchor: usize,
    location: Location,
    tag_location: Location,
}

/// An open collection.
enum Frame {
    Sequence(Header, Vec<RawNode>),
    Mapping(Header, Vec<(RawNode, RawNode)>, Option<RawNode>),
}

/// Builds the raw tree while the events arrive.
#[derive(Default)]
struct Composer {
    open: Vec<Frame>,
    root: Option<RawNode>,
    documents: usize,
}

/// Composes the only document of `text`. An empty stream is a null scalar at line 1, column 1.
///
/// # Errors
///
/// `E0A-02` when the parser rejects the text (including an alias to an unknown anchor) and
/// `E0A-03` at the start of a second document.
pub(crate) fn compose(text: &str) -> Result<RawNode, ReadError> {
    let mut composer = Composer::default();
    for event in Parser::new_from_str(text) {
        let (event, span) = event.map_err(|error| parse_error(&error))?;
        composer.accept(event, &span)?;
    }
    Ok(composer.root.unwrap_or_else(empty_document))
}

impl Composer {
    fn accept(&mut self, event: Event<'_>, span: &Span) -> Result<(), ReadError> {
        match event {
            Event::DocumentStart(..) => return self.start_document(span),
            Event::Scalar(text, style, anchor, tag) => {
                let header = header(anchor, tag.as_deref(), span);
                let plain = style == ScalarStyle::Plain;
                let content = RawContent::Scalar {
                    text: text.into_owned(),
                    plain,
                };
                self.attach(finish(header, content));
            }
            Event::Alias(anchor) => {
                let header = header(0, None, span);
                self.attach(finish(header, RawContent::Alias(anchor)));
            }
            Event::SequenceStart(_, anchor, tag) => {
                let header = header(anchor, tag.as_deref(), span);
                self.open.push(Frame::Sequence(header, Vec::new()));
            }
            Event::MappingStart(_, anchor, tag) => {
                let header = header(anchor, tag.as_deref(), span);
                self.open.push(Frame::Mapping(header, Vec::new(), None));
            }
            Event::SequenceEnd | Event::MappingEnd => self.close(),
            _ => {}
        }
        Ok(())
    }

    fn start_document(&mut self, span: &Span) -> Result<(), ReadError> {
        self.documents += 1;
        if self.documents > 1 {
            return Err(ReadError::new(
                ReadErrorCode::SeveralDocuments,
                location(span.start),
                "the stream holds more than one YAML document",
            ));
        }
        Ok(())
    }

    fn close(&mut self) {
        let node = match self.open.pop() {
            Some(Frame::Sequence(header, items)) => finish(header, RawContent::Sequence(items)),
            Some(Frame::Mapping(header, entries, _)) => {
                finish(header, RawContent::Mapping(entries))
            }
            None => return,
        };
        self.attach(node);
    }

    /// Adds a finished node to the open collection, or makes it the root.
    fn attach(&mut self, node: RawNode) {
        match self.open.last_mut() {
            None => self.root = Some(node),
            Some(Frame::Sequence(_, items)) => items.push(node),
            Some(Frame::Mapping(_, entries, pending_key)) => match pending_key.take() {
                Some(key) => entries.push((key, node)),
                None => *pending_key = Some(node),
            },
        }
    }
}

fn header(anchor: usize, tag: Option<&Tag>, span: &Span) -> Header {
    let content_location = location(span.start);
    Header {
        tag: tag.map_or(RawTag::Absent, raw_tag),
        anchor,
        location: content_location,
        tag_location: span.tag_start().map_or(content_location, location),
    }
}

fn finish(header: Header, content: RawContent) -> RawNode {
    RawNode {
        content,
        tag: header.tag,
        anchor: header.anchor,
        location: header.location,
        tag_location: header.tag_location,
    }
}

fn raw_tag(tag: &Tag) -> RawTag {
    if tag.handle().is_empty() && tag.suffix() == "!" {
        return RawTag::NonSpecific;
    }
    let name = format!("{}{}", tag.handle(), tag.suffix());
    match name.strip_prefix(STANDARD_TAG_PREFIX) {
        Some(suffix) => RawTag::Standard(suffix.to_owned()),
        None => RawTag::Other(name),
    }
}

fn empty_document() -> RawNode {
    let start = Location { line: 1, column: 1 };
    RawNode {
        content: RawContent::Scalar {
            text: String::new(),
            plain: true,
        },
        tag: RawTag::Absent,
        anchor: 0,
        location: start,
        tag_location: start,
    }
}

/// The parser counts lines from 1 and columns from 0.
fn location(marker: Marker) -> Location {
    Location {
        line: marker.line(),
        column: marker.col() + 1,
    }
}

fn parse_error(error: &ScanError) -> ReadError {
    ReadError::new(
        ReadErrorCode::Unparsable,
        location(*error.marker()),
        error.info(),
    )
}
