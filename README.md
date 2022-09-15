# avremu
An ATtiny1626 emulator designed for the [QUTy Development Board](https://cab202.github.io/quty/).

## Usage
```sh
./avremu <firmware file> <events file> <clock cycle limit> [debug]
```

**Example:** To run firmware.hex and process though events.txt for a half a second of instructions (at 3.33Mhz) run the following.
```sh
./avremu firmware.hex events.txt 1666667
```

### Events
Each line in the events file must follow the following format.
```
@<time in cycles> <device>: <event>
```

**Example:** To turn the potentiometer at a quarter from the LOW most position, press S1 after 128 clock cycles, release S1 after 128 more clock cycles (256 total), refer to the example below.
```
@00000000 R1: 0.250
@00000080 S1: PRESS
@00000100 S1: RELEASE
```