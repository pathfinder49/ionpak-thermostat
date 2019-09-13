use logos::Logos;
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
    #[token = "enable"]
    Enable,
    #[token = "disable"]
    Disable,
    #[token = "setup"]
    Setup,
    #[token = "ref+"]
    RefPos,
    #[token = "ref-"]
    RefNeg,
    #[token = "ain+"]
    AinPos,
    #[token = "ain-"]
    AinNeg,
    #[token = "unipolar"]
    Unipolar,
    #[token = "burnout"]
    Burnout,

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
pub enum ShowCommand {
    ReportMode,
}

#[derive(Debug)]
pub enum Command {
    Quit,
    Show(ShowCommand),
    Report(ReportMode),
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
        ]
    }
}
