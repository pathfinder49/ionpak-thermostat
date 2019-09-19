use core::fmt;
use nom::{
    IResult,
    branch::alt,
    bytes::complete::{is_a, tag, take_while1},
    character::{is_digit, complete::{char, one_of}},
    combinator::{complete, map, value},
    sequence::{preceded, separated_pair},
    multi::{fold_many0, fold_many1},
    error::ErrorKind,
};
use lexical_core as lexical;


#[derive(Clone, Debug)]
pub enum Error {
    Parser(ErrorKind),
    Incomplete,
    UnexpectedInput(u8),
    ParseNumber(lexical::Error)
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

impl From<lexical::Error> for Error {
    fn from(e: lexical::Error) -> Self {
        Error::ParseNumber(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Error::Incomplete =>
                "incomplete input".fmt(fmt),
            Error::UnexpectedInput(c) => {
                "unexpected input: ".fmt(fmt)?;
                c.fmt(fmt)
            }
            Error::Parser(e) => {
                "parser: ".fmt(fmt)?;
                (e as &dyn core::fmt::Debug).fmt(fmt)
            }
            Error::ParseNumber(e) => {
                "parsing number: ".fmt(fmt)?;
                (e as &dyn core::fmt::Debug).fmt(fmt)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum ShowCommand {
    Input,
    Reporting,
    Pwm,
    Pid,
    PostFilter,
}

#[derive(Debug, Clone)]
pub enum PidParameter {
    Target,
    KP,
    KI,
    KD,
    OutputMin,
    OutputMax,
    IntegralMin,
    IntegralMax,
}

#[derive(Debug, Clone)]
pub enum PwmMode {
    Manual {
        width: u32,
        total: u32,
    },
    Pid,
}

#[derive(Debug, Clone)]
pub enum Command {
    Quit,
    Show(ShowCommand),
    Reporting(bool),
    Pwm {
        channel: usize,
        mode: PwmMode,
    },
    Pid {
        channel: usize,
        parameter: PidParameter,
        value: f32,
    },
    PostFilter {
        channel: usize,
        rate: f32,
    },
}

fn end(input: &[u8]) -> IResult<&[u8], ()> {
    complete(
        fold_many0(
            one_of("\r\n\t "),
            (), |(), _| ()
        )
    )(input)
}

fn whitespace(input: &[u8]) -> IResult<&[u8], ()> {
    fold_many1(char(' '), (), |(), _| ())(input)
}

fn unsigned(input: &[u8]) -> IResult<&[u8], Result<u32, Error>> {
    take_while1(is_digit)(input)
        .map(|(input, digits)| {
            let result = lexical::parse(digits)
                .map_err(|e| e.into());
            (input, result)
        })
}

fn float(input: &[u8]) -> IResult<&[u8], Result<f32, Error>> {
    let (input, sign) = is_a("-")(input)?;
    let negative = sign.len() > 0;
    let (input, digits) = take_while1(|c| is_digit(c) || c == '.' as u8)(input)?;
    let result = lexical::parse(digits)
        .map(|result: f32| if negative { -result } else { result })
        .map_err(|e| e.into());
    Ok((input, result))
}

fn off_on(input: &[u8]) -> IResult<&[u8], bool> {
    alt((value(false, tag("off")),
         value(true, tag("on"))
    ))(input)
}

fn channel(input: &[u8]) -> IResult<&[u8], usize> {
    map(one_of("01"), |c| (c as usize) - ('0' as usize))(input)
}

fn report(input: &[u8]) -> IResult<&[u8], Command> {
    preceded(
        tag("report"),
        alt((
            preceded(
                whitespace,
                preceded(
                    tag("mode"),
                    alt((
                        preceded(
                            whitespace,
                            // `report mode <on | off>` - Switch repoting mode
                            map(off_on, Command::Reporting)
                        ),
                        // `report mode` - Show current reporting state
                        value(Command::Show(ShowCommand::Reporting), end)
                    ))
                )),
            // `report` - Report once
            value(Command::Show(ShowCommand::Input), end)
        ))
    )(input)
}

/// `pwm <0-1> <width> <total>` - Set pwm duty cycle
fn pwm_manual(input: &[u8]) -> IResult<&[u8], Result<PwmMode, Error>> {
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
    Ok((input, Ok(PwmMode::Manual { width, total })))
}

/// `pwm <0-1> pid` - Set PWM to be controlled by PID
fn pwm_pid(input: &[u8]) -> IResult<&[u8], Result<PwmMode, Error>> {
    value(Ok(PwmMode::Pid), tag("pid"))(input)
}

fn pwm(input: &[u8]) -> IResult<&[u8], Result<Command, Error>> {
    let (input, _) = tag("pwm")(input)?;
    alt((
        preceded(
            whitespace,
            map(
                separated_pair(
                    channel,
                    whitespace,
                    alt((
                        pwm_pid,
                        pwm_manual,
                    ))
                ),
                |(channel, mode)| mode.map(|mode| Command::Pwm { channel, mode })
            )
        ),
        value(Ok(Command::Show(ShowCommand::Pwm)), end)
    ))(input)
}

/// `pid <0-1> <parameter> <value>`
fn pid_parameter(input: &[u8]) -> IResult<&[u8], Result<Command, Error>> {
    let (input, channel) = channel(input)?;
    let (input, _) = whitespace(input)?;
    let (input, parameter) =
        alt((value(PidParameter::Target, tag("target")),
             value(PidParameter::KP, tag("kp")),
             value(PidParameter::KI, tag("ki")),
             value(PidParameter::KD, tag("kd")),
             value(PidParameter::OutputMin, tag("output_min")),
             value(PidParameter::OutputMax, tag("output_max")),
             value(PidParameter::IntegralMin, tag("integral_min")),
             value(PidParameter::IntegralMax, tag("integral_max"))
        ))(input)?;
    let (input, _) = whitespace(input)?;
    let (input, value) = float(input)?;
    let result = value
        .map(|value| Command::Pid { channel, parameter, value });
    Ok((input, result))
}

/// `pid` | pid_parameter
fn pid(input: &[u8]) -> IResult<&[u8], Result<Command, Error>> {
    let (input, _) = tag("pid")(input)?;
    alt((
        preceded(
            whitespace,
            pid_parameter
        ),
        value(Ok(Command::Show(ShowCommand::Pid)), end)
    ))(input)
}

fn postfilter(input: &[u8]) -> IResult<&[u8], Result<Command, Error>> {
    let (input, _) = tag("postfilter")(input)?;
    alt((
        preceded(
            whitespace,
            |input| {
                let (input, channel) = channel(input)?;
                let (input, _) = whitespace(input)?;
                let (input, _) = tag("rate")(input)?;
                let (input, _) = whitespace(input)?;
                let (input, rate) = float(input)?;
                let result = rate
                    .map(|rate| Command::PostFilter {
                        channel, rate,
                    });
                Ok((input, result))
            }
        ),
        value(Ok(Command::Show(ShowCommand::PostFilter)), end)
    ))(input)
}

fn command(input: &[u8]) -> IResult<&[u8], Result<Command, Error>> {
    alt((value(Ok(Command::Quit), tag("quit")),
         map(report, Ok),
         pwm,
         pid,
         postfilter,
    ))(input)
}

impl Command {
    pub fn parse(input: &[u8]) -> Result<Self, Error> {
        match command(input) {
            Ok((b"", result)) =>
                result,
            Ok((input_remain, _)) =>
                Err(Error::UnexpectedInput(input_remain[0])),
            Err(e) =>
                Err(e.into()),
        }
    }
}
