# iLO

Rust libraries and tools for interacting with HP iLO 2,3,4.  This is still early in development and may contain critical bugs.

## Overview

ilo is a set of libraries and tool
iLO is a set of libraries and tools to interact with the hardware controller or integrated lights out
(iLO version 2,3,4) in HP ProLiant servers. These controllers allows you to do things like check the
hardware status, power cycle the server and run a KVM remote console.

## Tools

The tools are built with statically compiled libraries using [Vcpkg](https://github.com/microsoft/vpkg).  To
install the required libraries you need the [cargo-vcpkg](https://crates.io/crates/cargo-vcpkg) which can be
installed with the following command.

```
cargo install cargo-vcpkg
```

To install the statically compiled libraries required to compile the tools use the following command

```
cargo vcpkg build
```

### console

[![Demo of console](https://videos.vortorus.net/videos/ilo-console.gif)](https://www.youtube.com/watch?v=WBbkc5Nt--s)

A tool for connecting to iLO 2 and displaying a virtual screen using using sdl2 with the contents that
would be displayed if a monitor was connected to the server.  It currently allows keyboard but not mouse
input.  To access the console you need to set the ip address and create an account through the iLO boot menu.
The tool requires you to put the ip address, username and password you set in the *auth.json* json file
as shown bellow.

```json
{
  "hostname": "ILO-IPADDRESS",
  "username": "ILO-USERNAME",
  "password": "ILO-PASSWORD"
}
```

you can the run the console with

```
cargo run --release --bin console --auth auth.json
```

## RIBCL tools

The remaining tools require credentials in a json file named *endpoint.json* as with the following structure.

```json
{
  "auth": {
    "hostname": "ILO-IPADDRESS",
    "username": "ILO-USERNAME",
    "password": "ILO-PASSWORD"
  }
}
```

### dump

a tool for sending raw RIBCL xml command files

command.xml
```xml
<?xml version="1.0"?>
<ribcl version="2.0">
    <login user_login="ILO-USERNAME" password="ILO-PASSWORD">
        <server_info mode="read">
            <get_embedded_health/>
        </server_info>
    </login>
</ribcl>
```

```
cargo run --release --bin dump -- --endpoint endpoint.json command.xml
```

### info
a tool to dump all information about and node

```
cargo run --release --bin info -- --endpoint endpoint.json
```

### power
a tool to turn on/off power on a server

```
cargo run --release --bin power -- --cold-boot
```