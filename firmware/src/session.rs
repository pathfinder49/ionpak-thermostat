use core::ops::Deref;
use core::fmt;
use super::command_parser::{Command, Error as ParserError};

const MAX_LINE_LEN: usize = 64;

struct LineReader {
    buf: [u8; MAX_LINE_LEN],
    pos: usize,
}

impl LineReader {
    pub fn new() -> Self {
        LineReader {
            buf: [0; MAX_LINE_LEN],
            pos: 0,
        }
    }

    pub fn feed(&mut self, c: u8) -> Option<LineResult> {
        if c == 13 || c == 10 {
            // Enter
            if self.pos > 0 {
                let len = self.pos;
                self.pos = 0;
                Some(LineResult {
                    buf: self.buf.clone(),
                    len,
                })
            } else {
                None
            }
        } else if self.pos < self.buf.len() {
            // Add input
            self.buf[self.pos] = c;
            self.pos += 1;
            None
        } else {
            // Buffer is full, ignore
            None
        }
    }
}

pub struct LineResult {
    buf: [u8; MAX_LINE_LEN],
    len: usize,
}

impl Deref for LineResult {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        &self.buf[..self.len]
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ReportMode {
    Off,
    Once,
    Continuous,
}

impl fmt::Display for ReportMode {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            ReportMode::Off => "off",
            ReportMode::Once => "once",
            ReportMode::Continuous => "continuous",
        }.fmt(fmt)
    }
}

pub enum SessionOutput {
    Nothing,
    Command(Command),
    Error(ParserError),
}

impl From<Result<Command, ParserError>> for SessionOutput {
    fn from(input: Result<Command, ParserError>) -> Self {
        input.map(SessionOutput::Command)
            .unwrap_or_else(SessionOutput::Error)
    }
}

pub struct Session {
    reader: LineReader,
    report_mode: ReportMode,
    report_pending: bool,
}

impl Session {
    pub fn new() -> Self {
        Session {
            reader: LineReader::new(),
            report_mode: ReportMode::Off,
            report_pending: false,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.reader.pos > 0
    }

    pub fn report_mode(&self) -> ReportMode {
        self.report_mode
    }

    pub fn set_report_pending(&mut self) {
        self.report_pending = true;
    }

    pub fn is_report_pending(&self) -> bool {
        match self.report_mode {
            ReportMode::Off => false,
            _ => self.report_pending,
        }
    }

    pub fn mark_report_sent(&mut self) {
        self.report_pending = false;
        match self.report_mode {
            ReportMode::Once =>
                self.report_mode = ReportMode::Off,
            _ => {}
        }
    }

    pub fn feed(&mut self, buf: &[u8]) -> (usize, SessionOutput) {
        let mut buf_bytes = 0;
        for (i, b) in buf.iter().enumerate() {
            buf_bytes = i + 1;
            let line = self.reader.feed(*b);
            match line {
                Some(line) => {
                    let command = Command::parse(&line);
                    match command {
                        Ok(Command::Report(mode)) => {
                            self.report_mode = mode;
                        }
                        _ => {}
                    }
                    return (buf_bytes, command.into());
                }
                None => {}
            }
        }
        (buf_bytes, SessionOutput::Nothing)
    }
}
