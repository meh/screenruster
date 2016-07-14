ScreenRuster
============
An X11 screen saver and locker.

Installation
------------
To install the daemon you will need a nightly Rust toolchain, then you can
install it with Cargo:

```shell
cargo install screenruster
```

Once that's done you can create a configuration file at
`$XDG_CONFIG_HOME/screenruster/config.toml` (`$XDG_CONFIG_HOME` defaults to
`~/.config/`) or copy one from `assets/config.toml`.

Screen savers have to be in `$PATH` and the executable name has to start with
`screenruster-saver-`.

A sample screen saver can be installed with Cargo:

```shell
cargo install screenruster-saver-laughing_man
```

The sample configuration file already has default settings for it.

Usage
-----

First, start the daemon.

```
screenruster &
```

Then if you want to activate the screen saver manually:

```
screenruster activate
```

Or if you want to lock manually:

```
screenruster lock
```

To unlock, simply type your password and press enter.

Available savers
================
This is a list of available screen savers that will be updated over time, if
you made a saver and want it added here just open a pull request.

- [Laughing Man](https://github.com/meh/screenruster-saver-laughing_man) from Ghost in the Shell: Stand Alone Complex

Architecture
============
The architecture loosely follows xscreensaver  because it's the best approach,
keep the screen locking simple and delegate the fancy graphics to a separate
proccess, this has the nice property of making buggy savers not bring down the
whole locking mechanism, thus protecting from vulnerabilities.

The savers can be written in any language that can draw to an X11 window, parse
and generate JSON, write to `stdout` and read from `stdin`.

JSON is used for IPC between the daemon and the saver, the daemon writes to the
saver process `stdin`, reads from the process `stdout` and forwards anything
coming from `stderr` to allow for debugging or logging.

The job of the saver is merely to do the rendering, this includes any fade
in/out or dialog boxes, this further reduces the attack surface of the locker.
__Note that the saver does not actually get the input, it just gets `Insert` or `Delete` events, so
it can fill its dialog box.__

Protocol
========
The protocol is line based, where each line contains a JSON encoded message,
each message has a `type` field with the name of the message, the parameters
are attributes on the same object.

Requests
--------
Requests are messages sent from the daemon to the spawned saver process.

### Configuration

The configuration request is part of the handshake and it's the first request sent when
a process is spawned.

The configuration is monolithic and managed by the daemon in a TOML file, the
related TOML map is converted to JSON and sent to the saver.

- `type`   = `"config"`
- `config` = `Object`

### Target

The target request is part of the handshake and is the second request sent when
a process is spawned.

It contains the details required to get the X11 window, the display name, the
screen number and the window `XID`.

- `type`    = `"target"`
- `display` = `String`
- `screen`  = `Integer`
- `window`  = `Integer`

### Resize

The resize request is sent when a locker window is resized, this can happen if
XRandr is used to change resolution or rotate the screen.

- `type`   = `"resize"`
- `width`  = `Integer`
- `height` = `Integer`

### Throttle

The throttle request is sent when the saver should try and reduce power usage.

- `type`     = `"throttle"`
- `throttle` = `Boolean`

### Blank

The blank request is sent when the screen has been blanked or unblanked.

- `type`     = `"blank"`
- `throttle` = `Boolean`

### Pointer

The pointer request is sent when a pointer event on the saver window has happened.

- `type`    = `"pointer"`
- `move`    = `Object { x, y }`
- `button` = `Object { x, y, button, press }`

### Password

The password request is sent when any authorization related changes happened,
this includes when characters are being inserted or deleted, the password is
being checked or authorization failed or succeded.

- `type`     = `"password"`
- `password` = `"insert"`, `"delete"`, `"reset"`, `"check"`, `"success"`, `"failure"`

### Start

The start request is sent when the saver should start its rendering, this may
include a fade in or other fancy graphics.

- `type` = `"start"`

### Stop

The stop request is sent when the saver should stop its rendering, this may
include a fade out or other fancy graphics.

- `type` = `"stop"`

Responses
---------
Responses are messages sent from the spawned saver process to the daemon.

### Initialized

The initialized response is sent after the handshake is done and the saver is
ready to start, since fancy graphics may require loading textures and such, the
saver is given some leeway to get ready to render.

- `type` = `"initialized"`

### Started

The started response is sent after a `start` request has been received and the
saver started its rendering, it tells the daemon it can show the window.

- `type` = `"started"`

### Stopped

The stopped response is sent after a `stop` request has been received and the
saver stopped its rendering, it tells the daemon it can hide the window.

Authorization
=============
Authorization is handled by various modules, each module tries to authenticate, the first
successful authentication unlocks the screen.

Internal
--------
The internal module uses a password specified in the configuration file, this
was initially made for testing, and you should probably not use it.

```toml
[auth.internal]
password = "password"
```

PAM
---
This module uses the Pluggable Authentication Module for authentication, you
will need to install a configuration file for it in `/etc/pam.d/screenruster`.

```config
auth	include		system-auth
```

If you want PAM account management to be respected, make sure to build with the
`auth-pam-accounts` feature.
