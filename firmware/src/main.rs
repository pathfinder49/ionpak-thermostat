#![feature(const_fn, proc_macro_hygiene)]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

use cortex_m_rt::entry;
use core::fmt::{self, Write};
use smoltcp::time::Instant;
use smoltcp::wire::{IpCidr, IpAddress, EthernetAddress};
use smoltcp::iface::{NeighborCache, EthernetInterfaceBuilder};
use smoltcp::socket::{SocketSet, TcpSocket, TcpSocketBuffer};
use cortex_m_semihosting::hio;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        write!($crate::UART0, $($arg)*).unwrap()
    })
}

#[macro_export]
macro_rules! println {
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

#[cfg(not(test))]
#[no_mangle] // https://github.com/rust-lang/rust/issues/{38281,51647}
#[panic_handler]
pub fn panic_fmt(info: &core::panic::PanicInfo) -> ! {
    println!("{}", info);
    let mut stdout = hio::hstdout().unwrap();
    let _ = writeln!(stdout, "{}", info);
    loop {}
}

mod board;
use self::board::{
    gpio::Gpio,
    systick::get_time,
};
mod ethmac;
mod command_parser;
use command_parser::{Command, ShowCommand, PwmSetup, PwmMode, PwmConfig};
mod session;
use self::session::{Session, SessionOutput};
mod ad7172;
mod pid;
mod tec;
use tec::{Tec, TecPin};

pub struct UART0;

impl fmt::Write for UART0 {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        let uart_0 = unsafe { &*tm4c129x::UART0::ptr() };
        for c in s.bytes() {
            while uart_0.fr.read().txff().bit() {}
            uart_0.dr.write(|w| w.data().bits(c))
        }
        Ok(())
    }
}

const TCP_RX_BUFFER_SIZE: usize = 256;
const TCP_TX_BUFFER_SIZE: usize = 8192;


macro_rules! create_socket_storage {
    ($rx_storage:ident, $tx_storage:ident) => (
        let mut $rx_storage = [0; TCP_RX_BUFFER_SIZE];
        let mut $tx_storage = [0; TCP_TX_BUFFER_SIZE];
    )
}

macro_rules! create_socket {
    ($set:ident, $rx_storage:ident, $tx_storage:ident, $target:ident) => (
        let tcp_rx_buffer = TcpSocketBuffer::new(&mut $rx_storage[..]);
        let tcp_tx_buffer = TcpSocketBuffer::new(&mut $tx_storage[..]);
        let tcp_socket = TcpSocket::new(tcp_rx_buffer, tcp_tx_buffer);
        let $target = $set.add(tcp_socket);
    )
}

const DEFAULT_PID_PARAMETERS: pid::Parameters = pid::Parameters {
    kp: 1.0,
    ki: 1.0,
    kd: 1.0,
    output_min: 0.0,
    output_max: 0xffff as f32,
    integral_min: 0.0,
    integral_max: 0xffff as f32,
};

const PWM_PID_WIDTH: u16 = 0xffff;

// TODO: maybe rename to `TECS`?
/// Number of TEC channels with four PWM channels each
pub const CHANNELS: usize = 2;

// TODO: maybe rename to `TecState`?
/// State per TEC channel
#[derive(Clone)]
struct ControlState {
    /// Report data (time, data)
    report: Option<(u64, u32)>,
    pid_enabled: bool,
    pid: pid::Controller,
}

#[cfg(not(test))]
#[entry]
fn main() -> ! {
    let mut stdout = hio::hstdout().unwrap();
    writeln!(stdout, "tecpak boot").unwrap();
    board::init();
    writeln!(stdout, "board initialized").unwrap();
    let mut tec0 = Tec::tec0();
    let mut tec1 = Tec::tec1();

    println!(r#"
  _                         _
 | |                       | |
/  _/___  _ __  _ __   __ _| |
 | |/ _ \ /'__\| '_ \ / _` | |/ /
 | | (/_/| |___| |_) | (_| |   <
 |_|\___\ \___/| .__/ \__,_|_|\_\
               | |
               |_|             v1
"#);
    // CSn
    let pb4 = board::gpio::PB4.into_output();
    // SCLK
    let pb5 = board::gpio::PB5.into_output();
    // MOSI
    let pe4 = board::gpio::PE4.into_output();
    // MISO
    let pe5 = board::gpio::PE5.into_input();
    // max 2 MHz = 0.5 us
    let mut delay_fn = || for _ in 0..10 { cortex_m::asm::nop(); };
    let spi = board::softspi::SyncSoftSpi::new(
        board::softspi::SoftSpi::new(pb5, pe4, pe5),
        &mut delay_fn
    );
    let mut adc = ad7172::Adc::new(spi, pb4).unwrap();
    loop {
        let r = adc.identify();
        match r {
            Err(e) =>
                writeln!(stdout, "Cannot identify ADC: {:?}", e).unwrap(),
            Ok(id) if id & 0xFFF0 == 0x00D0 => {
                writeln!(stdout, "ADC id: {:04X}", id).unwrap();
                break;
            }
            Ok(_id) => {
                // This always happens on the first attempt. So retry silently
            }
        };
    }
    writeln!(stdout, "AD7172: setting checksum mode").unwrap();
    adc.set_checksum_mode(ad7172::ChecksumMode::Crc).unwrap();
    loop {
        let r = adc.identify();
        match r {
            Err(e) =>
                writeln!(stdout, "Cannot identify ADC: {:?}", e).unwrap(),
            Ok(id) if id & 0xFFF0 == 0x00D0 => {
                writeln!(stdout, "ADC id: {:04X}", id).unwrap();
                break;
            }
            Ok(id) =>
                writeln!(stdout, "Corrupt ADC id: {:04X}", id).unwrap(),
        };
    }
    adc.set_sync_enable(false).unwrap();
    // SENS0_{P,N}
    adc.setup_channel(0, ad7172::Input::Ain0, ad7172::Input::Ain1).unwrap();
    // SENS1_{P,N}
    adc.setup_channel(1, ad7172::Input::Ain2, ad7172::Input::Ain3).unwrap();

    let init_state = ControlState {
        report: None,
        // Start with disengaged PID to let user setup parameters first
        pid_enabled: false,
        pid: pid::Controller::new(DEFAULT_PID_PARAMETERS.clone()),
    };
    let mut states = [init_state.clone(), init_state.clone()];

    let mut hardware_addr = EthernetAddress(board::get_mac_address());
    writeln!(stdout, "MAC address: {}", hardware_addr).unwrap();
    if hardware_addr.is_multicast() {
        writeln!(stdout, "programmed MAC address is invalid, using default").unwrap();
        hardware_addr = EthernetAddress([0x10, 0xE2, 0xD5, 0x00, 0x03, 0x00]);
    }
    let mut ip_addrs = [IpCidr::new(IpAddress::v4(192, 168, 1, 26), 24)];
    println!("MAC {} IP {}", hardware_addr, ip_addrs[0]);
    let mut neighbor_cache_storage = [None; 8];
    let neighbor_cache = NeighborCache::new(&mut neighbor_cache_storage[..]);
    let mut device = ethmac::Device::new();
    unsafe { device.init(hardware_addr) };
    let mut iface = EthernetInterfaceBuilder::new(&mut device)
                .ethernet_addr(hardware_addr)
                .neighbor_cache(neighbor_cache)
                .ip_addrs(&mut ip_addrs[..])
                .finalize();

    create_socket_storage!(tcp_rx_storage0, tcp_tx_storage0);
    create_socket_storage!(tcp_rx_storage1, tcp_tx_storage1);
    create_socket_storage!(tcp_rx_storage2, tcp_tx_storage2);
    create_socket_storage!(tcp_rx_storage3, tcp_tx_storage3);
    create_socket_storage!(tcp_rx_storage4, tcp_tx_storage4);
    create_socket_storage!(tcp_rx_storage5, tcp_tx_storage5);
    create_socket_storage!(tcp_rx_storage6, tcp_tx_storage6);
    create_socket_storage!(tcp_rx_storage7, tcp_tx_storage7);

    let mut socket_set_entries: [_; 8] = Default::default();
    let mut sockets = SocketSet::new(&mut socket_set_entries[..]);

    create_socket!(sockets, tcp_rx_storage0, tcp_tx_storage0, tcp_handle0);
    create_socket!(sockets, tcp_rx_storage1, tcp_tx_storage1, tcp_handle1);
    create_socket!(sockets, tcp_rx_storage2, tcp_tx_storage2, tcp_handle2);
    create_socket!(sockets, tcp_rx_storage3, tcp_tx_storage3, tcp_handle3);
    create_socket!(sockets, tcp_rx_storage4, tcp_tx_storage4, tcp_handle4);
    create_socket!(sockets, tcp_rx_storage5, tcp_tx_storage5, tcp_handle5);
    create_socket!(sockets, tcp_rx_storage6, tcp_tx_storage6, tcp_handle6);
    create_socket!(sockets, tcp_rx_storage7, tcp_tx_storage7, tcp_handle7);
    let mut sessions_handles = [
        (Session::new(), tcp_handle0),
        (Session::new(), tcp_handle1),
        (Session::new(), tcp_handle2),
        (Session::new(), tcp_handle3),
        (Session::new(), tcp_handle4),
        (Session::new(), tcp_handle5),
        (Session::new(), tcp_handle6),
        (Session::new(), tcp_handle7),
    ];

    loop {
        // ADC input
        adc.data_ready()
            .unwrap_or_else(|e| {
                writeln!(stdout, "ADC error: {:?}", e).unwrap();
                None
            }).map(|channel| {
                let now = get_time();
                let data = adc.read_data().unwrap();
                let state = &mut states[usize::from(channel)];

                if state.pid_enabled {
                    let width = state.pid.update(data as f32) as u16;
                    match channel {
                        0 => tec0.set(TecPin::ISet, width, PWM_PID_WIDTH),
                        1 => tec1.set(TecPin::ISet, width, PWM_PID_WIDTH),
                        _ => unreachable!(),
                    }
                }

                state.report = Some((now, data));
                for (session, _) in sessions_handles.iter_mut() {
                    session.set_report_pending(channel.into());
                }
            });

        for (session, tcp_handle) in sessions_handles.iter_mut() {
            let socket = &mut *sockets.get::<TcpSocket>(*tcp_handle);
            if !socket.is_open() {
                if session.is_dirty() {
                    // Reset a previously uses session/socket
                    *session = Session::new();
                }
                socket.listen(23).unwrap()
            }

            if socket.may_recv() && socket.may_send() {
                let output = socket.recv(|buf| session.feed(buf));

                // TODO: use "{}" to display pretty errors
                match output {
                    Ok(SessionOutput::Nothing) => {}
                    Ok(SessionOutput::Command(command)) => match command {
                        Command::Quit =>
                            socket.close(),
                        Command::Reporting(reporting) => {
                            let _ = writeln!(socket, "Report mode set to {}", if reporting { "on" } else { "off" });
                        }
                        Command::Show(ShowCommand::Reporting) => {
                            let _ = writeln!(socket, "Report mode: {}", if session.reporting() { "on" } else { "off" });
                        }
                        Command::Show(ShowCommand::Input) => {
                            for (channel, state) in states.iter().enumerate() {
                                state.report.map(|(time, data)| {
                                    let _ = writeln!(socket, "t={}, sens{}={}", time, channel, data);
                                });
                            }
                        }
                        Command::Show(ShowCommand::Pid) => {
                            for (channel, state) in states.iter().enumerate() {
                                let _ = writeln!(socket, "PID settings for channel {}", channel);
                                let pid = &state.pid;
                                let _ = writeln!(socket, "- target={:.4}", pid.get_target());
                                let p = pid.get_parameters();
                                macro_rules! out {
                                    ($p: tt) => {
                                        let _ = writeln!(socket, "* {}={:.4}", stringify!($p), p.$p);
                                    };
                                }
                                out!(kp);
                                out!(ki);
                                out!(kd);
                                out!(output_min);
                                out!(output_max);
                                out!(integral_min);
                                out!(integral_max);
                            }
                        }
                        Command::Show(ShowCommand::Pwm) => {
                            for (channel, state) in states.iter().enumerate() {
                                let _ = writeln!(
                                    socket, "PWM {}: PID {}",
                                    channel,
                                    if state.pid_enabled { "engaged" } else { "disengaged" }
                                );
                                for pin in TecPin::VALID_VALUES {
                                    let (width, total) = match channel {
                                        0 => tec0.get(*pin),
                                        1 => tec1.get(*pin),
                                        _ => unreachable!(),
                                    };
                                    let _ = writeln!(socket, "- {}={}/{}", pin, width, total);
                                }
                            }
                        }
                        Command::Show(ShowCommand::PostFilter) => {
                            for (channel, _) in states.iter().enumerate() {
                                match adc.get_postfilter(channel as u8).unwrap() {
                                    Some(filter) => {
                                        let _ = writeln!(
                                            socket, "channel {}: postfilter={:.2} SPS",
                                            channel, filter.output_rate().unwrap()
                                        );
                                    }
                                    None => {
                                        let _ = writeln!(
                                            socket, "channel {}: no postfilter",
                                            channel
                                        );
                                    }
                                }
                            }
                        }
                        Command::Pwm { channel, setup: PwmSetup::ISet(PwmMode::Pid) } => {
                            states[channel].pid_enabled = true;
                            let _ = writeln!(socket, "channel {}: PID enabled to control PWM", channel);
                        }
                        Command::Pwm { channel, setup: PwmSetup::ISet(PwmMode::Manual(config))} => {
                            states[channel].pid_enabled = false;
                            let PwmConfig { width, total } = config;
                            match channel {
                                0 => tec0.set(TecPin::ISet, width, total),
                                1 => tec1.set(TecPin::ISet, width, total),
                                _ => unreachable!(),
                            }
                            let _ = writeln!(
                                socket, "channel {}: PWM duty cycle manually set to {}/{}",
                                channel, config.width, config.total
                            );
                        }
                        Command::Pwm { channel, setup } => {
                            let (pin, config) = match setup {
                                PwmSetup::ISet(_) =>
                                    // Handled above
                                    unreachable!(),
                                PwmSetup::MaxIPos(config) =>
                                    (TecPin::MaxIPos, config),
                                PwmSetup::MaxINeg(config) =>
                                    (TecPin::MaxINeg, config),
                                PwmSetup::MaxV(config) =>
                                    (TecPin::MaxV, config),
                            };
                            let PwmConfig { width, total } = config;
                            match channel {
                                0 => tec0.set(pin, width, total),
                                1 => tec1.set(pin, width, total),
                                _ => unreachable!(),
                            }
                        }
                        Command::Pid { channel, parameter, value } => {
                            let pid = &mut states[channel].pid;
                            use command_parser::PidParameter::*;
                            match parameter {
                                Target =>
                                    pid.set_target(value),
                                KP =>
                                    pid.update_parameters(|parameters| parameters.kp = value),
                                KI =>
                                    pid.update_parameters(|parameters| parameters.ki = value),
                                KD =>
                                    pid.update_parameters(|parameters| parameters.kd = value),
                                OutputMin =>
                                    pid.update_parameters(|parameters| parameters.output_min = value),
                                OutputMax =>
                                    pid.update_parameters(|parameters| parameters.output_max = value),
                                IntegralMin =>
                                    pid.update_parameters(|parameters| parameters.integral_min = value),
                                IntegralMax =>
                                    pid.update_parameters(|parameters| parameters.integral_max = value),
                            }
                            let _ = writeln!(socket, "PID parameter updated");
                        }
                        Command::PostFilter { channel, rate } => {
                            let filter = ad7172::PostFilter::closest(rate);
                            match filter {
                                Some(filter) => {
                                    adc.set_postfilter(channel as u8, Some(filter)).unwrap();
                                    let _ = writeln!(
                                        socket, "channel {}: postfilter set to {:.2} SPS",
                                        channel, filter.output_rate().unwrap()
                                    );
                                }
                                None => {
                                    let _ = writeln!(socket, "Unable to choose postfilter");
                                }
                            }
                        }
                    }
                    Ok(SessionOutput::Error(e)) => {
                        let _ = writeln!(socket, "Command error: {:?}", e);
                    }
                    Err(_) => {}
                }
            }
            if socket.may_send() {
                if let Some(channel) = session.is_report_pending() {
                    states[channel].report.map(|(time, data)| {
                        let _ = writeln!(socket, "t={} sens{}={:06X}", time, channel, data);
                    });
                    session.mark_report_sent(channel);
                }
            }
        }
        match iface.poll(&mut sockets, Instant::from_millis((get_time() / 1000) as i64)) {
            Ok(_) => (),
            Err(e) => println!("poll error: {}", e)
        }
    }
}
