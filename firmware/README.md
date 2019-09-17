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
