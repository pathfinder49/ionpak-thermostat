#![feature(const_fn)]
#![no_std]
#![no_main]

use cortex_m_rt::entry;
use core::fmt::{self, Write};
use embedded_hal::blocking::delay::DelayUs;
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
use self::board::gpio::Gpio;
mod ethmac;
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
    let mut delay = unsafe { board::delay::Delay::new() };
    // CSn
    let pb4 = board::gpio::PB4.into_output();
    // SCLK
    let pb5 = board::gpio::PB5.into_output();
    // MOSI
    let pe4 = board::gpio::PE4.into_output();
    // MISO
    let pe5 = board::gpio::PE5.into_input();
    // max 2 MHz = 0.5 us
    let mut delay_fn = || delay.delay_us(1u32);
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
    let handles = [
        tcp_handle0,
        tcp_handle1,
        tcp_handle2,
        tcp_handle3,
        tcp_handle4,
        tcp_handle5,
        tcp_handle6,
        tcp_handle7,
    ];

    let mut time = 0i64;
    let mut data = None;
    // if a socket has sent the latest data
    let mut socket_pending = [false; 8];
    loop {
        adc.data_ready()
            .and_then(|channel|
                channel.map(|channel|
                    adc.read_data().map(|new_data| {
                        data = Some(Ok((channel, new_data)));
                        for p in socket_pending.iter_mut() {
                            *p = true;
                        }
                    })
                ).unwrap_or(Ok(()))
            )
            .map_err(|e| {
                data = Some(Err(e));
                for p in socket_pending.iter_mut() {
                    *p = true;
                }

            });
        for (&tcp_handle, pending) in handles.iter().zip(socket_pending.iter_mut()) {
            let socket = &mut *sockets.get::<TcpSocket>(tcp_handle);
            if !socket.is_open() {
                socket.listen(23).unwrap()
            }

            if socket.may_send() && *pending {
                match &data {
                    Some(Ok((channel, input))) => {
                        let _ = writeln!(socket, "channel={} input={}\r", channel, input);
                    }
                    Some(Err(ad7172::AdcError::ChecksumMismatch(Some(expected), Some(input)))) => {
                        let _ = writeln!(socket, "checksum_expected={:02X} checksum_input={:02X}\r", expected, input);
                    }
                    Some(Err(e)) => {
                        let _ = writeln!(socket, "adc_error={:?}\r", e);
                    }
                    None => {}
                }
                *pending = false;
            }
        }
        match iface.poll(&mut sockets, Instant::from_millis(time)) {
            Ok(_) => (),
            Err(e) => println!("poll error: {}", e)
        }
        time += 1;
    }
}
