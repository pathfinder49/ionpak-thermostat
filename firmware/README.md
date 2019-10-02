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

| Syntax                                    | Function                                                   |
| ---                                       | ---                                                        |
| `report`                                  | Show current input                                         |
| `report mode`                             | Show current report mode                                   |
| `report mode <off/on>`                    | Set report mode                                            |
| `pwm <0/1> max_i_pos <width> <total>`     | Set PWM duty cycle for **max_i_pos** to *width / total*    |
| `pwm <0/1> max_i_neg <width> <total>`     | Set PWM duty cycle for **max_i_neg** to *width / total*    |
| `pwm <0/1> max_v <width> <total>`         | Set PWM duty cycle for **max_v** to *width / total*        |
| `pwm <0/1> <width> <total>`               | Set PWM duty cycle for **i_set** to manual *width / total* |
| `pwm <0/1> pid`                           | Set PWM to be controlled by PID                            |
| `pid`                                     | Show PID configuration                                     |
| `pid <0/1> target <value>`                | Set the PID controller target                              |
| `pid <0/1> kp <value>`                    | Set proportional gain                                      |
| `pid <0/1> ki <value>`                    | Set integral gain                                          |
| `pid <0/1> kd <value>`                    | Set differential gain                                      |
| `pid <0/1> output_min <value>`            | Set mininum output                                         |
| `pid <0/1> output_max <value>`            | Set maximum output                                         |
| `pid <0/1> integral_min <value>`          | Set integral lower bound                                   |
| `pid <0/1> integral_max <value>`          | Set integral upper bound                                   |
| `s-h`                                     | Show Steinhart-Hart equation parameters                    |
| `s-h <0/1> <a/b/c> <value>`               | Set Steinhart-Hart parameter for a channel                 |
| `s-h <0/1> <parallel_resistance> <value>` | Set parallel resistance of the ADC                         |
| `postfilter <0/1> rate <rate>`            | Set postfilter output data rate                            |
