use logos::{Logos, Lexer};
use super::session::ReportMode;

#[derive(Logos, Debug, PartialEq)]
enum Token {
    #[end]
    End,
    #[error]
    Error,

    #[token = "Quit"]
    Quit,
    #[token = "show"]
    Show,
    #[token = "channel"]
    Channel,
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

    #[regex = "[0-9]+"]
    Number,
}

#[derive(Debug)]
pub enum Error {
    Parser,
    UnexpectedEnd,
    UnexpectedToken(Token),
}

#[derive(Debug)]
pub enum CommandShow {
    ReportMode,
}

#[derive(Debug)]
pub enum Command {
    Quit,
    Show(CommandShow),
    Report(ReportMode),
}



impl Command {
    pub fn parse(input: &str) -> Result<Self, Error> {
        let mut lexer = Token::lexer(input);

        macro_rules! choice {
            [$($token: tt => $block: stmt,)*] => {
                match lexer.token {
                    $(
                        Token::$token => {
                            lexer.advance();
                            $block
                        }
                    )*
                    Token::End => Err(Error::UnexpectedEnd),
                    _ => Err(Error::UnexpectedToken(lexer.token))
                }
            }
        }

        choice![
            Quit => Ok(Command::Quit),
            Report => choice![
                Mode => choice![
                    End => Ok(Command::Show(CommandShow::ReportMode)),
                    Off => Ok(Command::Report(ReportMode::Off)),
                    Once => Ok(Command::Report(ReportMode::Once)),
                    Continuous => Ok(Command::Report(ReportMode::Continuous)),
                ],
                End => Ok(Command::Report(ReportMode::Once)),
            ],
        ]
    }
}
