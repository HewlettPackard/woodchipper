# Installation

Pre-built binaries for various platforms may be found on the
[releases page][releases].

To install the latest release, try:

 * **Linux x86_64**: [direct link to latest version][linux]

   To install the latest tagged release:
   ```bash
   sudo curl -SsLf \
    -o /usr/local/bin/woodchipper \
    "https://github.com/HewlettPackard/woodchipper/releases/latest/download/woodchipper-x86_64-unknown-linux-musl"

   sudo chmod +x /usr/local/bin/woodchipper
   ```

   Alternatively, to install a particular tagged version:
   ```bash
   version="v1.0.0"
   url="https://github.com/HewlettPackard/woodchipper/releases/download/$version/woodchipper-x86_64-unknown-linux-musl"

   sudo curl -SsLf "$url" -o /usr/local/bin/woodchipper
   sudo chmod +x /usr/local/bin/woodchipper
   ```

   ... or you can manually save a release to a location on your `$PATH` and mark
   it as executable (`chmod +x ...`).

   > **Note:** this static build requires [xclip] for copy and paste support. If
   you prefer native clipboard integration, see the "Install via Cargo" steps
   below.

 * **Windows x86_86**: [direct link to latest version][windows]

   Place the `.exe` somewhere convenient or on your `$PATH`.

 * **macOS**: [direct link to the latest version][macos]

   To install the latest tagged release:
   ```bash
   sudo curl -SsLf \
    -o /usr/local/bin/woodchipper \
    "https://github.com/HewlettPackard/woodchipper/releases/latest/download/woodchipper-x86_64-apple-darwin"

   sudo chmod +x /usr/local/bin/woodchipper
   ```

   Alternatively, to install a particular tagged version:

   ```bash
   version="v1.0.0"
   url="https://github.com/HewlettPackard/woodchipper/releases/download/$version/woodchipper-x86_64-apple-darwin"

   sudo curl -SsLf "$url" -o /usr/local/bin/woodchipper
   sudo chmod +x /usr/local/bin/woodchipper
   ```
  
 * **Install via Cargo**:

   To build and install manually using Cargo (probably installed via [rustup]):

   ```bash
   cargo install woodchipper
   ```

   See the platform notes below for any specific install requirements,
   particularly on Linux.

[releases]: https://github.com/HewlettPackard/woodchipper/releases
[linux]: https://github.com/HewlettPackard/woodchipper/releases/download/latest/woodchipper-x86_64-unknown-linux-musl
[windows]: https://github.com/HewlettPackard/woodchipper/releases/download/latest/woodchipper-x86_64-pc-windows-gnu.exe
[macos]: https://github.com/HewlettPackard/woodchipper/releases/latest/download/woodchipper-x86_64-apple-darwin
[xclip]: https://github.com/astrand/xclip
[rustup]: https://rustup.rs/

## Platform Notes

 * **All platforms**
   * Custom color output (base16 colors) requires 256 color support. This may
     mean setting your `$TERM` to `xterm-256color` or so; the specific steps
     for doing so depend on your particular terminal.

 * **Linux**
   * Statically-linked builds require `xclip` for clipboard support.
   * Dynamically-linked builds (`cargo build`) using the gnu toolchain will
     write to the clipboard natively, but require additional packages to build.

     On Debian and Ubuntu, this means:

     ```bash
     sudo apt-get install \
       xorg-dev python3 \
       libxcb-shape0-dev libxcb-xfixes0-dev libxcb-render0-dev
     ```

     ...these may be uninstalled after building if desired.

 * **Windows**
   * Styled output requires full ANSI color support.

     The Windows 10 terminal supports this, but older built-in Windows
     terminals do not.

     Older versions of the Windows command-prompt aren't supported, at least
     for the styled and interactive renderers, though the plaintext renderers
     (`-r plain` or `-r json`) should work fine. Alternative terminal emulators
     like [ConEmu] may work but aren't tested.
 
     On Windows, only the Windows 10 terminal properly supports styled output.
     Older versions may need to use an alternative terminal or fall back to the
     plain or JSON renderers (`-r plain`, `-r json`)

 * **macOS**
   * Consider using iTerm2 and enabling the "Scroll wheel sends arrow keys when
     in alternative screen mode" option under Preferences -> Advanced -> Mouse.
 
[ConEmu]: https://github.com/Maximus5/ConEmu
