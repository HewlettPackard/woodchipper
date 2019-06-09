# Installation

Pre-built binaries for various platforms may be found on the
[releases page][releases].

To install the latest release, try:

 * **Linux x86_64**: [direct link to latest version][linux]

   Woodchipper may be installed with the following bash snippet:
   ```bash
   version="latest"
   url="https://github.com/HewlettPackard/woodchipper/releases/download/$version/woodchipper-x86_64-unknown-linux-musl"

   sudo curl -SsLF  -o /usr/local/bin/woodchipper
   chmod +x /usr/local/bin/woodchipper
   ```

   ... or manually 
 * **Windows x86_86**: [direct link to latest version][windows]

   Place the `.exe` somewhere convenient or on your `$PATH`.

 * **macOS:** pre-built binaries coming soon!

[releases]: https://github.com/HewlettPackard/woodchipper/releases
[linux]: https://github.com/HewlettPackard/woodchipper/releases/download/latest/woodchipper-x86_64-unknown-linux-musl
[windows]: https://github.com/HewlettPackard/woodchipper/releases/download/latest/woodchipper-x86_64-pc-windows-gnu.exe

## Platform Notes

 * **All platforms**
   * Custom color output (base16 colors) requires 256 color support. This may
     mean setting your `$TERM` to `xterm-256color` or so; the specific steps
     for doing so depend on your particular terminal.

 * **Linux**
   * Statically-linked builds require `xclip` for clipboard support.
   * Dynamically-linked builds (`cargo build`) using the gnu toolchain will
     write to the clipboard natively, but require additional packages to build.

     On Debian, this means:

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
 
[ConEmu]: https://github.com/Maximus5/ConEmu

   



