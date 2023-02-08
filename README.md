# avremu

Emulator for the ATtiny1626 microcontroller and QUTy development board. Developed for support of learning and teaching in *CAB202 Microprocessors and Digital Systems*.

Authored by Dr Mark Broadmeadow (mark.broadmeadow@qut.edu.au).

```
Usage: avremu.exe [OPTIONS] <FIRMWARE>

Arguments:
  <FIRMWARE>  Microcontroller firmware to load in .HEX format

Options:
  -e, --events <EVENTS>    Specify event file for hardware events
  -t, --timeout <TIMEOUT>  Specify emulation runtime limit in nanoseconds
  -s, --dump-stack         Dump stack to stdout on termination
  -r, --dump-regs          Dump working register values to stdout on termination
  -o, --dump-stdout        Dump output of stdio pseudo-peripheral to file stdout.txt on termination
  -d, --debug              Enable debug output
  -h, --help               Print help
  -V, --version            Print version
  ```