use std::{
    fmt::Display,
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

#[derive(Debug)]
pub struct SiteLog {
    pub messages: Vec<LogMessage>,
}

#[derive(Debug)]
pub enum LogMessage {
    // LoadIC::Load_NOT_FOUND
    // Runtime::GetObjectProperty_NOT_FOUND
    UndefinedProperty {
        name: String,
        source: PropertySource,
        stack_trace: String,
    },
    // StoreIC::Store
    // Runtime::SetObjectProperty_TAINTED
    AssignTaintedKey {
        class_name: String,
        key: String,
        value: String,
        source: PropertySource,
        stack_trace: String,
    },
    // LogFrameLocation
    Location {
        url: String,
    },
    // Runtime::SetObjectProperty_PROTOTYPE
    Polluted {
        key: String,
        value: String,
        stack_trace: String,
    },
    // From_JS + JS Prototype Get
    PrototypeGet {
        key: String,
        value: String,
        stack_trace: String,
    },
    // LogIfStringTainted
    SinkReached {
        sink_type: String,
        value: String,
        stack_trace: String,
    },
    // From_JS + DOCUMENT_START
    DocumentStart,
}

#[derive(Debug)]
pub enum PropertySource {
    InlineCache,
    RuntimeObject,
}

#[derive(Debug)]
pub enum LogError {
    Io(std::io::Error),
    Utf8(std::string::FromUtf8Error),
    IntParse(std::num::ParseIntError),
}

impl From<std::io::Error> for LogError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<std::string::FromUtf8Error> for LogError {
    fn from(value: std::string::FromUtf8Error) -> Self {
        Self::Utf8(value)
    }
}

impl From<std::num::ParseIntError> for LogError {
    fn from(value: std::num::ParseIntError) -> Self {
        Self::IntParse(value)
    }
}

impl Display for LogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for LogError {}

pub fn parse_log(path: &Path) -> Result<SiteLog, LogError> {
    let file = File::open(path)?;
    let buf_reader = BufReader::new(file);

    let mut messages: Vec<LogMessage> = Vec::new();

    let mut iter = buf_reader.bytes();

    'outer: loop {
        loop {
            match iter.next() {
                Some(b) => {
                    if b? == b'[' {
                        break;
                    }
                }
                None => break 'outer,
            }
        }
        let name = read_until_whitespace(&mut iter)?;

        macro_rules! handle_undefined_property {
            ($source: expr) => {
                let property = read_sized_string(&mut iter)?;
                let stack_trace = read_sized_string(&mut iter)?;
                messages.push(LogMessage::UndefinedProperty {
                    name: property,
                    source: $source,
                    stack_trace,
                });
            };
        }

        macro_rules! handle_assign_tainted_key {
            ($source: expr) => {
                let class_name = read_sized_string(&mut iter)?;
                let key = read_sized_string(&mut iter)?;
                let value = read_sized_string(&mut iter)?;
                let stack_trace = read_sized_string(&mut iter)?;
                messages.push(LogMessage::AssignTaintedKey {
                    class_name,
                    key,
                    value,
                    source: $source,
                    stack_trace,
                })
            };
        }

        match name.as_str() {
            "LoadIC::Load_NOT_FOUND]" => {
                handle_undefined_property!(PropertySource::InlineCache);
            }
            "Runtime::GetObjectProperty_NOT_FOUND]" => {
                handle_undefined_property!(PropertySource::RuntimeObject);
            }
            "StoreIC::Store]" => {
                handle_assign_tainted_key!(PropertySource::InlineCache);
            }
            "Runtime::SetObjectProperty_TAINTED]" => {
                handle_assign_tainted_key!(PropertySource::RuntimeObject);
            }
            "LogFrameLocation]" => {
                let new_location = read_sized_string(&mut iter)?;
                messages.push(LogMessage::Location { url: new_location });
            }
            "Runtime::SetObjectProperty_PROTOTYPE]" => {
                let key = read_sized_string(&mut iter)?;
                let value = read_sized_string(&mut iter)?;
                let stack_trace = read_sized_string(&mut iter)?;
                messages.push(LogMessage::Polluted {
                    key,
                    value,
                    stack_trace,
                })
            }
            "From_JS]" => {
                let msg_type = read_until_whitespace(&mut iter)?;
                match msg_type.as_str() {
                    "DOCUMENT_LOAD" => {
                        messages.push(LogMessage::DocumentStart);
                    }
                    "PROTOTYPE_GET" => {
                        let key = read_sized_string(&mut iter)?;
                        let value = read_sized_string(&mut iter)?;
                        let stack_trace = read_sized_string(&mut iter)?;
                        messages.push(LogMessage::PrototypeGet {
                            key,
                            value,
                            stack_trace,
                        })
                    }
                    _ => {}
                }
                // TODO
            }
            "LogIfStringTainted]" => {
                let sink_type = read_until_whitespace(&mut iter)?;
                let value = read_sized_string(&mut iter)?;
                let stack_trace = read_sized_string(&mut iter)?;
                messages.push(LogMessage::SinkReached {
                    sink_type,
                    value,
                    stack_trace,
                })
            }
            _ => {}
        }
    }

    Ok(SiteLog { messages })
}

fn read_until_whitespace(
    iter: &mut impl Iterator<Item = Result<u8, std::io::Error>>,
) -> Result<String, LogError> {
    let str_bytes: Vec<u8> = iter
        .take_while(|c| {
            c.as_ref()
                .ok()
                .map(|c| !c.is_ascii_whitespace())
                .unwrap_or(false)
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(String::from_utf8(str_bytes)?)
}

fn read_sized_string(
    iter: &mut impl Iterator<Item = Result<u8, std::io::Error>>,
) -> Result<String, LogError> {
    let size: usize = read_until_whitespace(iter)?.parse()?;
    let str_bytes: Vec<u8> = iter.take(size).collect::<Result<Vec<_>, _>>()?;

    // take byte (space) after string
    iter.next();

    Ok(String::from_utf8(str_bytes)?)
}
