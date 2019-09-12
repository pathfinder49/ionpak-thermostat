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

    #[regex = "[0-9]+"]
    Number,
}

#[derive(Debug)]
pub enum Error {
    Parser,
    UnexpectedEnd,
    UnexpectedToken(Token),
    NoSuchChannel,
    NoSuchSetup,
}

#[derive(Debug)]
pub enum ShowCommand {
    ReportMode,
}


#[derive(Debug)]
pub enum ChannelCommand {
    Enable,
    Disable,
    Setup(u8),
}

#[derive(Debug)]
pub enum Command {
    Quit,
    Show(ShowCommand),
    Report(ReportMode),
    Channel(u8, ChannelCommand),
}

const CHANNEL_IDS: &'static [&'static str] = &[
    "0", "1", "2", "3",
];
const SETUP_IDS: &'static [&'static str] = CHANNEL_IDS;

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
                    Token::End => Err(Error::UnexpectedEnd),
                    _ => Err(Error::UnexpectedToken(lexer.token))
                }
            };
        }
        /// Expecting no further tokens
        macro_rules! end {
            ($result: expr) => {
                match lexer.token {
                    Token::End => Ok($result),
                    _ => Err(Error::UnexpectedToken(lexer.token)),
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
            Channel => choice![
                Number => {
                    let channel = CHANNEL_IDS.iter()
                        .position(|id| *id == lexer.slice());
                    match channel {
                        Some(channel) => {
                            choice![
                                Enable => Ok(Command::Channel(
                                    channel as u8,
                                    ChannelCommand::Enable
                                )),
                                Disable => Ok(Command::Channel(
                                    channel as u8,
                                    ChannelCommand::Enable
                                )),
                                Setup => choice![
                                    Number => {
                                        let setup = SETUP_IDS.iter()
                                            .position(|id| *id == lexer.slice());
                                        match setup {
                                            Some(setup) =>
                                                end!(Command::Channel(
                                                    channel as u8,
                                                    ChannelCommand::Setup(setup as u8)
                                                )),
                                            None =>
                                                Err(Error::NoSuchSetup)
                                        }
                                    },
                                ],
                            ]
                        }
                        None => Err(Error::NoSuchChannel)
                    }
                },
            ],
        ]
    }
}
