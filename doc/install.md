# Installation

Pre-built binaries for various platforms may be found on the
[releases page][releases].

## Platform Notes

 * macOS builds are Coming Soon
 * Styled output requires ANSI color support.
 
   On Windows, only the Windows 10 terminal properly supports styled output.
   Older versions may need to use an alternative terminal or fall back to the
   plain or JSON renderers (`-r plain`, `-r json`)
 * Custom color output (base16 colors) requires 256 color support.
 * Statically-linked Linux builds require `xclip` for clipboard support.
 * Dynamically-linked Linux builds (`cargo build`) using the gnu toolchain will
   write to the clipboard natively, but require additional packages to build.

   On Debian:

   ```bash
   sudo apt-get install \
     xorg-dev python3 \
     libxcb-shape0-dev libxcb-xfixes0-dev libxcb-render0-dev
   ```

   These may be uninstalled after building if desired.

[releases]: https://github.com/HewlettPackard/woodchipper/releases
