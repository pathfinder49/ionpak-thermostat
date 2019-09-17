# Thermostat v1 prototype firmware

## Building

### On Debian-based systems

- install [rustup](https://rustup.rs/)

```shell
apt install gcc gcc-arm-none-eabi git-core
rustup toolchain install nightly
rustup update
rustup target add thumbv7em-none-eabihf --toolchain nightly
rustup default nightly
rustup component add rust-src
cargo install cargo-xbuild
git clone https://github.com/llvm/llvm-project.git
export RUST_COMPILER_RT_ROOT=`pwd`/llvm-project/compiler-rt

cd firmware && cargo xbuild --release
```

The built ELF file will be at `target/thumbv7em-none-eabihf/release/ionpak-firmware`

### Development build on NixOS

Requires NixOS 19.09 or later for cargo-xbuild.

```shell
nix-shell --run "cd firmware && cargo xbuild --release"
```

## Network

### Setup

Ethernet, IP: 192.168.1.26/24

Use telnet or netcat to connect to port 23/tcp (telnet)

### Reading ADC input

Set report mode to `once` to obtain the single next value. Report mode
will turn itself off after the next reading.

Set report mode to `continuous` for a continuous stream of input data.

The scope of this setting is per TCP session.


### Commands

| Syntax                     | Function                                     |
| ---                        | ---                                          |
| `report`                   | Show current input                           |
| `report mode`              | Show current report mode                     |
| `report mode <off or on>`  | Set report mode                              |
| `pwm <width> <total>`      | Set PWM duty cycle to manual *width / total* |
| `pwm pid`                  | Set PWM to be controlled by PID              |
| `pid target <value>`       |                                              |
| `pid kp <value>`           |                                              |
| `pid ki <value>`           |                                              |
| `pid kd <value>`           |                                              |
| `pid output_min <value>`   |                                              |
| `pid output_max <value>`   |                                              |
| `pid integral_min <value>` |                                              |
| `pid integral_max <value>` |                                              |
