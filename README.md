libmonitor
===========

This crate aims to be a unified solution to interacting with display devices. It provides a `Monitor` class that can be used to set or read settings of a Monitor.

## Supported Operations:
- Read/Set Contrast
- Read/Set Brightness
- Read/Set On Screen Display Language
- Read/Set Monitor Input Source

## Lower Level Access

`libmonitor` also enables lower level access to the monitor communication bus. Per default type safe abstractions are used, but custom messages can also be sent and received.

## Supported DDC/CI Operations
- [x] Read Capabilities
- [x] Set/Read VcpValue
- [ ] Read Timing Report
- [ ] Set/Read VcpTable

## OS Support
- [x] Linux
- [ ] Windows
- [ ] MacOS


## Standards

Display device communication has been standardized by VESA. The current public standards can be found here:
[VESA Public Standards](https://app.box.com/s/vcocw3z73ta09txiskj7cnk6289j356b/folder/11133487793)

Relevant for this Library:
- *E-EDID*: Display Device Identification Data
- *E-DDC*: I2C-Bus device definition's and communication
- *DDCCI*: Display Command Interface for Display Setting Manipulation
- *MCCS*: Display Features and Capabilities Definitions

## Previous Work

This crate was build after finding previous solutions to be incomplete and fragmented. The following crates deserve an honorable mention for providing inspiration:
- [mccs](https://crates.io/crates/mccs)
- [ddc-hi](https://crates.io/crates/ddc-hi)
- [ddc-i2c](https://crates.io/crates/ddc-i2c)
- [ddc-macos](https://crates.io/crates/ddc-macos)
- [ddc-winapi](https://crates.io/crates/ddc-winapi)
- [edid](https://crates.io/crates/edid)
