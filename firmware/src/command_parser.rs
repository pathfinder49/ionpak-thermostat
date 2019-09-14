use logos::Logos;
use btoi::{btoi, ParseIntegerError};
use super::session::ReportMode;

#[derive(Logos, Debug, PartialEq)]
enum Token {
    #[end]
    End,
    #[error]
    Error,

    #[token = "Quit"]
    Quit,
    #[token = "report"]
    Report,
    #[token = "mode"]
    Mode,
    #[token = "off"]
    Off,
    #[token = "once"]
    Once,
    #[token = "continuous"]
    Continuous,
    #[token = "pwm"]
    Pwm,

    #[regex = "[0-9]+"]
    Number,
}

#[derive(Debug)]
pub enum Error {
    Parser,
    UnexpectedEnd,
    UnexpectedToken(Token),
    ParseInteger(ParseIntegerError)
}

impl From<ParseIntegerError> for Error {
    fn from(e: ParseIntegerError) -> Self {
        Error::ParseInteger(e)
    }
}

#[derive(Debug)]
pub enum ShowCommand {
    ReportMode,
}

#[derive(Debug)]
pub enum Command {
    Quit,
    Show(ShowCommand),
    Report(ReportMode),
    Pwm {
        pwm_match: u32,
        pwm_reload: u32,
    },
}

impl Command {
    pub fn parse(input: &str) -> Result<Self, Error> {
        let mut lexer = Token::lexer(input);

        /// Match against a set of expected tokens
        macro_rules! choice {
            [$($token: tt => $block: stmt,)*] => {
                match lexer.token {
                    $(
                        Token::$token => {
                            lexer.advance();
                            $block
                        }
                    )*
                    Token::End => return Err(Error::UnexpectedEnd),
                    _ => return Err(Error::UnexpectedToken(lexer.token))
                }
            };
        }
        /// Expecting no further tokens
        macro_rules! end {
            ($result: expr) => {
                match lexer.token {
                    Token::End => Ok($result),
                    _ => return Err(Error::UnexpectedToken(lexer.token)),
                }
            };
        }

        // Command grammar
        choice![
            Quit => Ok(Command::Quit),
            Report => choice![
                Mode => choice![
                    End => end!(Command::Show(ShowCommand::ReportMode)),
                    Off => Ok(Command::Report(ReportMode::Off)),
                    Once => Ok(Command::Report(ReportMode::Once)),
                    Continuous => Ok(Command::Report(ReportMode::Continuous)),
                ],
                End => Ok(Command::Report(ReportMode::Once)),
            ],
            Pwm => {
                if lexer.token != Token::Number {
                    return Err(Error::UnexpectedToken(lexer.token));
                }
                let pwm_match = btoi(lexer.slice().as_bytes())?;
                lexer.advance();

                if lexer.token != Token::Number {
                    return Err(Error::UnexpectedToken(lexer.token));
                }
                let pwm_reload = btoi(lexer.slice().as_bytes())?;
                lexer.advance();

                if lexer.token != Token::End {
                    return Err(Error::UnexpectedToken(lexer.token));
                }

                end!(Command::Pwm {
                    pwm_match, pwm_reload,
                })
            },
        ]
    }
}
