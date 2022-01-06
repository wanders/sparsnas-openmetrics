OpenMetrics/Prometheus exporter for IKEA sparsnäs energy monitor
================================================================

Receiver for the IKEA sparsnäs energy monitor using RFM69 radio
module. Exposes the data in openmetrics format (that prometheus can
scrape).

Usage
-----

```
USAGE:
    sparsnas-openmetrics [OPTIONS] <SERIAL>

ARGS:
    <SERIAL>

OPTIONS:
    -h, --help                                                 Print help information
    -o, --openmetrics-port <OPENMETRICS_PORT>                  [default: 3030]
        --openmetrics-listen-addr <OPENMETRICS_LISTEN_ADDR>    [default: ::]
    -p, --pulses-per-kwh <PULSES_PER_KWH>                      [default: 1000]
    -v, --verbose
    -V, --version                                              Print version information
```

An example unit file for systemd (sparsnas-openmetrics.service) is
included, which needs to be modified to have correct serial number
before linking it.

Hardware
--------

This expects to run on a Raspberry Pi with an RFM69 module (868MHz
variant!) connected to the SPI interface on the gpio header.


| RFM69 name | RPi pin         |
|------------|-----------------|
| `RESET`    | 29  `GPIO 05`   |
| `DIO0`     | 18  `GPIO 24`   |
| `3.3V`     | 17              |
| `NSS`      | 24  `SPI_CE0_N` |
| `MOSI`     | 19  `SPI_MOSI`  |
| `MISO`     | 21  `SPI_MISO`  |
| `SCK`      | 23  `SPI_CLK`   |
| `ANTENNA`  |                 |
| `GND`      | 25              |



https://www.electrokit.com/produkt/rfm69hcw-868mhz-transceiver/


Cross compiling
---------------

`cross build --target=armv7-unknown-linux-gnueabihf`


License
-------

[Apache License, Version 2.0](COPYING)
