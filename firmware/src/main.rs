#![feature(const_fn)]
#![no_std]
#![no_main]

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

#[no_mangle] // https://github.com/rust-lang/rust/issues/{38281,51647}
#[panic_handler]
pub fn panic_fmt(info: &core::panic::PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

mod board;
use self::board::{gpio::Gpio, systick::get_time};
mod ethmac;
mod command_parser;
use command_parser::{Command, CommandShow};
mod session;
use self::session::{Session, SessionOutput};
mod ad7172;

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

#[entry]
fn main() -> ! {
    let mut stdout = hio::hstdout().unwrap();
    writeln!(stdout, "ionpak boot").unwrap();
    board::init();
    writeln!(stdout, "board initialized").unwrap();

    println!(r#"
  _                         _
 (_)                       | |
  _  ___  _ __  _ __   __ _| |
 | |/ _ \| '_ \| '_ \ / _` | |/ /
 | | (_) | | | | |_) | (_| |   <
 |_|\___/|_| |_| .__/ \__,_|_|\_\
               | |
               |_|
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
            Ok(id) =>
                writeln!(stdout, "Corrupt ADC id: {:04X}", id).unwrap(),
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

    let mut hardware_addr = EthernetAddress(board::get_mac_address());
    if hardware_addr.is_multicast() {
        println!("programmed MAC address is invalid, using default");
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

    let mut read_times = [0, 0];
    let mut data = None;
    // if a socket has sent the latest data
    loop {
        let _ = adc.data_ready()
            .and_then(|channel|
                channel.map(|channel|
                    adc.read_data().map(|new_data| {
                        let now = get_time();
                        read_times[0] = read_times[1];
                        read_times[1] = now;
                        data = Some((now, Ok((channel, new_data))));
                        for (session, _) in sessions_handles.iter_mut() {
                            session.set_report_pending();
                        }
                    })
                ).unwrap_or(Ok(()))
            )
            .map_err(|e| {
                let now = get_time();
                data = Some((now, Err(e)));
                for (session, _) in sessions_handles.iter_mut() {
                    session.set_report_pending();
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
                let command = socket.recv(|buf| session.feed(buf));

                match command {
                    Ok(SessionOutput::Nothing) => {}
                    Ok(SessionOutput::Command(Command::Quit)) =>
                        socket.close(),
                    Ok(SessionOutput::Command(Command::Report(mode))) => {
                        let _ = writeln!(socket, "Report mode: {:?}", mode);
                    }
                    Ok(SessionOutput::Command(Command::Show(CommandShow::ReportMode))) => {
                        let _ = writeln!(socket, "Report mode: {:?}", session.report_mode());
                    }
                    Ok(SessionOutput::Command(command)) => {
                        let _ = writeln!(socket, "Not implemented: {:?}", command);
                    }
                    Ok(SessionOutput::Error(e)) => {
                        let _ = writeln!(socket, "Command error: {:?}", e);
                    }
                    Err(_) => {}
                }
            }
            if socket.may_send() && session.is_report_pending() {
                match &data {
                    Some((time, Ok((channel, input)))) => {
                        let interval = read_times[1] - read_times[0];
                        let _ = writeln!(socket, "t={}-{} channel={} input={}\r", time, interval, channel, input);
                    }
                    Some((time, Err(ad7172::AdcError::ChecksumMismatch(Some(expected), Some(input))))) => {
                        let _ = writeln!(socket, "t={} checksum_expected={:02X} checksum_input={:02X}\r", time, expected, input);
                    }
                    Some((time, Err(e))) => {
                        let _ = writeln!(socket, "t={} adc_error={:?}\r", time, e);
                    }
                    None => {}
                }
                session.mark_report_sent();
            }
        }
        match iface.poll(&mut sockets, Instant::from_millis((get_time() / 1000) as i64)) {
            Ok(_) => (),
            Err(e) => println!("poll error: {}", e)
        }
    }
}
