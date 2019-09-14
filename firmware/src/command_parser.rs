use nom::{
    IResult,
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::{is_digit, complete::char},
    combinator::{map, value},
    sequence::{preceded, tuple, Tuple},
    multi::fold_many1,
    error::ErrorKind,
};
use btoi::{btoi, ParseIntegerError};
use super::session::ReportMode;


#[derive(Debug)]
pub enum Error {
    Parser(ErrorKind),
    Incomplete,
    UnexpectedInput(u8),
    ParseInteger(ParseIntegerError)
}

impl<'t> From<nom::Err<(&'t [u8], ErrorKind)>> for Error {
    fn from(e: nom::Err<(&'t [u8], ErrorKind)>) -> Self {
        match e {
            nom::Err::Incomplete(_) =>
                Error::Incomplete,
            nom::Err::Error((_, e)) =>
                Error::Parser(e),
            nom::Err::Failure((_, e)) =>
                Error::Parser(e),
        }
    }
}

impl From<ParseIntegerError> for Error {
    fn from(e: ParseIntegerError) -> Self {
        Error::ParseInteger(e)
    }
}

#[derive(Debug, Clone)]
pub enum ShowCommand {
    ReportMode,
}

#[derive(Debug, Clone)]
pub enum Command {
    Quit,
    Show(ShowCommand),
    Report(ReportMode),
    Pwm {
        width: u32,
        total: u32,
    },
}

fn whitespace(input: &[u8]) -> IResult<&[u8], ()> {
    fold_many1(char(' '), (), |(), _| ())(input)
}

fn unsigned(input: &[u8]) -> IResult<&[u8], Result<u32, ParseIntegerError>> {
    take_while1(is_digit)(input)
        .map(|(input, digits)| (input, btoi(digits)))
}

fn report_mode(input: &[u8]) -> IResult<&[u8], ReportMode> {
    alt((value(ReportMode::Off, tag("off")),
         value(ReportMode::Once, tag("once")),
         value(ReportMode::Continuous, tag("continuous"))
    ))(input)
}

fn report(input: &[u8]) -> IResult<&[u8], Command> {
    preceded(
        preceded(
            tag("report"),
            whitespace
        ),
        alt((
            preceded(
                whitespace,
                preceded(
                    tag("mode"),
                    alt((
                        preceded(
                            whitespace,
                            map(report_mode,
                                |mode| Command::Report(mode))
                        ),
                        |input| Ok((input, Command::Show(ShowCommand::ReportMode)))
                    ))
                )),
            |input| Ok((input, Command::Report(ReportMode::Once)))
        ))
    )(input)
}

fn pwm(input: &[u8]) -> IResult<&[u8], Result<Command, Error>> {
    let (input, _) = tag("pwm")(input)?;
    let (input, _) = whitespace(input)?;
    let (input, width) = unsigned(input)?;
    let width = match width {
        Ok(width) => width,
        Err(e) => return Ok((input, Err(e.into()))),
    };
    let (input, _) = whitespace(input)?;
    let (input, total) = unsigned(input)?;
    let total = match total {
        Ok(total) => total,
        Err(e) => return Ok((input, Err(e.into()))),
    };
    Ok((input, Ok(Command::Pwm { width, total })))
}

fn command(input: &[u8]) -> IResult<&[u8], Command> {
    alt((value(Command::Quit, tag("quit")),
         report
    ))(input)
}

impl Command {
    pub fn parse(input: &[u8]) -> Result<Self, Error> {
        match command(input) {
            Ok((b"", command)) =>
                Ok(command),
            Ok((input_remain, _)) =>
                Err(Error::UnexpectedInput(input_remain[0])),
            Err(e) =>
                Err(e.into()),
        }
    }
}
