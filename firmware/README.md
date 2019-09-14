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

| Syntax                                    | Function                            |
| ---                                       | ---                                 |
| `report`                                  | Report mode: once                   |
| `report mode`                             | Show current report mode            |
| `report mode <off or once or continuous>` | Set report mode                     |
| `pwm <width> <total>`                     | Set PWM duty cycle to width / total |

