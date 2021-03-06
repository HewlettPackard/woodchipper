# Design Overview

Woodchipper has a four-stop processing pipeline for incoming message:

 1. **Reading**: reads raw messages as strings from some source,
 2. **Parsing**: converts raw messages into a standardized format
 3. **Classification**: converts standardized messages into human-readable
    chunks with rendering metadata
 4. **Rendering:** displays messages to the screen, possibly applying styles and
    providing interactive features

Each stage may have multiple implementations and will be selected either by the
user (readers and renderers) or determined automatically (e.g. parsers and
classifiers).

## Reading

Readers fetch messages from some input source as text and pass them along for
parsing. Input sources may be local (stdin, file, subprocess) or may fetch log
messages via sockets or some API.

Existing implementations include:
 * [`stdin.rs`][stdin]: reads lines from standard input / pipes
 * [`stdin_hack.rs`][stdin_hack]: reads lines from `/dev/stdin` to avoid
   conflicts with the interactive renderer on Unix
 * [`null.rs`][null]: a dummy reader that prints an error and quits, used as a
   fallback if no other reader is available
 * the [kubernetes reader] fetches log messages from Kubernetes pods via
   the Kubernetes API

Readers run in a dedicated thread and send messages over a [channel] for further
processing. If needed, they may accept arguments via the [`Config`][config] to,
for example, set the Kubernetes namespace.

Rust's blocking IO means that reader threads cannot be reliably terminated at
users' request, so we can't necessarily expect readers to be capable of
responding to an exit request. However, readers require some cleanup actions may
use the optional exit request and response channels to listen for exit
requests, perform cleanup actions, and notify the main thread that it's safe to
terminate.

Rather than pushing just a raw message string over the channel, lines are
instead wrapped in a [`LogEntry`][renderer-types], allowing some additional
metadata to be send along the channel:

 * `LogEntry::eof()` can be sent to notify renderers that the end of input has
   been reached
 * `LogEntry::message()` is used to send normal messages
   
   Optionally, a [`ReaderMetadata`][parser-types] may be provided to pass along
   datatype hints if they're available at read-time, e.g. a source name if
   reading from multiple sources or a timestamp if tracked via the input api
   (e.g. Docker and Kubernetes).
 * `LogEntry::internal()` is used to send internal messages to the user as our
   own logging ability is restricted, particularly in the interactive renderer

[stdin]: ../../src/reader/stdin.rs
[stdin_hack]: ../../src/reader/stdin_hack.rs
[null]: ../../src/reader/null.rs
[kubernetes reader]: ./kubernetes-reader.md
[channel]: https://doc.rust-lang.org/std/sync/mpsc/fn.channel.html

## Parsing

Woodchipper parses lines independently to better support applications that
output multiple formats (e.g. startup scripts, 3rd party libraries, or multiple
separate Kubernetes containers). Parsers must quickly determine if messages are
supported or hand them off to the next parser in the chain.

If the parser can parse the input message, it returns a normalized
[`Message`][parser-types] instance with as much metadata as it could extract.

Existing implementations include:

 * [`json.rs`][json]: parses JSON log lines, i.e. lines like `{...}\n`

   It specifically aims to support [logrus][logrus-lib]-like JSON output
   formats, but various other field mappings are also supported.

   Prefers RFC-3339-style timestamps but falls back to [`dtparse`][dtparse].

   Unidentified fields are copied to the `metadata` field for use later in the
   pipline.
 * [`plain.rs`][plain]: the fallback parser; renders the raw message, but
   opportunistically includes metadata if it can be identified.

   Where possible, timestamps are parsed out of messages using
   [`dtparse`][dtparse], with some simple checks to discard timestamps for
   common failure cases. Log levels are identified where possible.

Parsers may refer to the reader's metadata to include or override their parsed
contextual info. For example, the plain parser prefers to use the reader's
timestamp rather than using the significantly slower and less accurate `dtparse`
free-form parser.

[logrus-lib]: https://github.com/sirupsen/logrus
[json]: ../../src/parser/json.rs
[plain]: ../../src/parser/plain.rs
[dtparse]: https://crates.io/crates/dtparse

## Classification

Given a normalized [`Message`][parser-types] instance, a classifier generates
some number of [`Chunks`][classifier-types]. They are responsible for
determining various rendering-specific attributes:

 * the formatted text content
 * the `kind`, used mainly for highlighting and aligning text segments
 * the `slot`, used to place the segment within a screen region (left, center,
   right)
 * the alignment of text within a chunk
 * padding, wrapping, and line break hints
 * the `weight`, used to hide less important chunks on smaller displays

At the moment, chunks are arranged based on the order in which classifiers are
executed. Chunks may contain children to individually apply styles to different
sub-sections of a text segment while avoiding improper line wrapping.

Classifiers may mark metadata fields as "consumed" by adding their keys to a
shared `HashSet`, allowing later classifiers in the chain to skip 

Existing implementations include:

 * [`timestamp.rs`][timestamp-classifier]: formats timestamps into two chunks,
   allowing the lower priority date chunk to be pruned while still displaying
   the time.
 * [`level.rs`][level-classifier]: adds the log level using its level-specific
   `kind`
 * [`text.rs`][text-classifier]: adds force-wrapped chunks per line of input
   text, allowing strings with newlines to be displayed sensibly
 * [`logrus.rs`][logrus-classifier]: extracts logrus's `file` field for display
   in the right column, trimming the path to the last few components
 * [`metadata.rs`][metadata-classifier]: adds all un-processed metadata fields
   to the message as `[key]=[value]` pairs

[timestamp-classifier]: ../src/classifier/timestamp.rs
[level-classifier]: ../src/classifier/level.rs
[text-classifier]: ../src/classifier/text.rs
[logrus-classifier]: ../src/classifier/logrus.rs
[metadata-classifier]: ../src/classifier/metadata.rs

## Rendering

Existing implementations include:

 * [`json.rs`][json-renderer]: writes the normalized parsed messages back to
   standard output, discarding classifier results. Useful for normalizing log
   messages in scripting applications.
 * [`plain.rs`][plain-renderer]: writes classified messages to standard output
   with basic (whitespace-only) formatting, suitable for sharing.

   This renderer is automatically selected if output is piped. The interactive
   renderer will re-format messages using this renderer when copying to the
   clipboard.
 * [`styled.rs`][styled-renderer]: writes classified and styled output to
   standard output.

   If terminal width can be detected, lines will be wrapped and a right-side
   column may display contextual information.

   This output is less suitable for sharing as it contains ANSI escape
   characters and right-aligned text.
 * the [interactive renderer]: a performant custom pager with interactive
   features, including text reflow, searching, filtering, and improved browsing.

[json-renderer]: ../../src/renderer/json.rs
[plain-renderer]: ../../src/renderer/plain.rs
[styled-renderer]: ../../src/renderer/styled.rs
[interactive renderer]: ./interactive-renderer.md

[config]: ../../src/config.rs
[renderer-types]: ../../src/renderer/types.rs
[parser-types]: ../../src/parser/types.rs
[classifier-types]: ../../src/classifier/types.rs
[renderer-types]: ../../src/renderer/types.rs
