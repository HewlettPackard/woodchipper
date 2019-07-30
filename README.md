# woodchipper

[![CircleCI](https://circleci.com/gh/HewlettPackard/woodchipper.svg?style=svg)](https://circleci.com/gh/HewlettPackard/woodchipper)

Follow, transform, and explore all your unwieldly microservice logs from the
terminal.

 * Ingests logs in any format, in real-time
 * Converts all logs to one of several unified formats:
    * formatted plain-text for sharing
    * stylized and wrapped for easy reading
    * JSON for machine processing
 * Interactive terminal interface adds painless searching, filtering, text
   reflow, and clipboard support
 * Built-in Kubernetes support follows multiple pods and containers at
   once
 * User-customizable output styles and custom log formats (see [customization])

## Quick Start

 1. Grab a pre-built binary from the [releases page][releases] or run:

    ```bash
    cargo install woodchipper
    ```

    See the [install page](./doc/install.md) for detailed instructions.

 2. Follow some logs:
    ```bash
    tail -f /var/log/my-app.log | woodchipper
    ```

 3. Use the [`kubectl` plugin][plugin] wrapper script and watch some pods:

    ```bash
    kubectl woodchipper -n my-namespace app=my-app
    ```

## Usage

Pipe any logs to `woodchipper`:

```bash
cat my-log.txt | woodchipper
```

This opens the interactive viewer by default. Use up, down, page up, page
down, home, and end to navigate.

woodchipper also follows any streaming output:
```bash
./some-long-running-script.sh | woodchipper
```

When piped, woodchipper automatically outputs nicely formatted plaintext,
appropriate for sharing:

```bash
./some-hard-to-read-json-logs.sh | woodchipper | cat
```

Alternatively, if you'd just like to print the colorized logs to your terminal:
```bash
./logs.sh | woodchipper -r styled
```

If you don't like the interactive viewer but still want a pager, try `less`:
```bash
cat logs.txt | woodchipper -r styled | less
```

(try `less -R` if your `less` doesn't pass through ANSI escapes by default)

### Interactive Viewer

The interactive viewer provides an improved pager with regex searching and
filtering. It's enabled by default if woodchipper is attached to a tty.

A number of keyboard shortcuts are available:

 * `up`, `down`: move the cursor one message at a time
 * `page up`, `page down`: scroll one screenful at a time
 * `home`, `end`: move to the start or end of all messages
 * `f`, `|`: add a filter to the stack
   * a filter regex may be freely entered
   * invalid filter regexes are highlighted in red
   * matching messages are highlighted as you type
   * `enter`: add the filter to the stack and remove all non-matching messages
   * `esc`: cancel filter
 * `p`: pop the last filter from the stack
 * `/`, `ctrl-f`: search for a particular message; when in filter mode:
   * a search regex may be freely entered
   * invalid search regexes are highlighted in red
   * all matching messages will be highlighted; the cursor will jump to the
     nearest forward match as you type
   * `enter`: next match
   * `ctrl-p`: previous match
   * `esc`: end search; if a result is highlighted, it will remain highlighted
 * `c`: copy the selected message to the clipboard as shareable plain text
 * `shift-c`: copy the current screen to the clipboard as shareable plain text
 * `q`: quit

The interactive viewer works best with terminal emulators that treat mouse wheel
input as up / down keypresses when in alternate screen mode. KDE's Konsole
behaves this way by default, and this may be enabled in iTerm2 in Preferences ->
Advanced -> Mouse -> "Scroll wheel sends arrow keys when in alternative screen
mode". An option to capture mouse events on all terminals may be added in the
future, however doing so disables text selection and isn't ideal.

### kubectl plugin

> *For `kubectl` 1.13+, [read more][kubectl-plugins]*

To make full use of the Kubernetes integration:

 * Ensure `kubectl` is available and configured on your `$PATH`
 * Install the [wrapper script][plugin] on your `$PATH`

Woodchipper uses `kubectl proxy` to access the Kubernetes API, so it can
connect to your cluster if `kubectl` can.

To follow a pod named `my-pod-1234`, run:
```bash
kubectl woodchipper -n my-namespace my-pod-1234
```

Alternatively, if you don't want to use the kubectl plugin, this is equivalent:
```bash
woodchipper --reader=kubernetes -n my-namespace my-pod-1234
```

Woodchipper matches pods continually using substrings, so a partial pod name
will follow pods even between restarts or deployment upgrades:
```bash
woodchipper --reader=kubernetes -n my-namespace my-pod
```

Multiple substrings can be used:
```bash
kubectl woodchipper -n my-namespace my-pod my-other-pod
```

Alternatively, if you give it a label-like selector, it will perform a label
query:
```bash
kubectl woodchipper -n my-namespace app=my-app
```

Note that only one label selector may be used at a time.

Woodchipper honors your configured `kubectl` default namespace, so you can
leave off `-n my-namespace` if `kubectl` is configured to use it already.
Alternatively, the `WD_NAMESPACE` environment variable can be set to override
the default.

[kubectl-plugins]: https://kubernetes.io/docs/tasks/extend-kubectl/kubectl-plugins/

## Supported Log Formats

Woodchipper can parse, format, and stylize any of the following logs,
potentially mixed together:

 * Several varieties of JSON logs, e.g. `{"time": "...", "msg": "hello world"}`
 * [logrus]-style key/value pair logs, e.g. `time="..." msg="hello world"`
 * [klog] logs for Kubernetes components
 * Plaintext logs with inferred timestamps and log levels
 * User-specified custom formats with the [regex parser][regex]

## Similar Projects

 * [stern] has similar Kubernetes tailing features
 * [logrus] has built-in pretty printing when a TTY is attached
 * [slog] provides structured pretty printing
 * [less] supports paging, searching, and input following

## Contributing

Bug reports, feature requests, and pull requests are welcome! Be sure to read
though the [code of conduct] for some pointers to get started.

Note that - as mentioned in the code of conduct - code contributions must
indicate that you accept the [Developer Certificate of Origin][dco],
essentially indicating you have rights to the code you're contributing and
that you agree to the project's license (MIT). With the Git CLI, simply pass
`-s` to `git commit`:

```bash
git commit -s [...]
```

... and Git will automatically append the required `Signed-off-by: ...` to the
end of your commit message.

Additionally, the [design documentation][design] may be a helpful resource for
understanding how woodchipper works.

[customization]: ./doc/customization.md
[plugin]: ./misc/kubectl-woodchipper
[releases]: https://github.com/HewlettPackard/woodchipper/releases/latest
[klog]: https://github.com/kubernetes/klog
[regex]: ./doc/customization.md#log-formats
[stern]: https://github.com/wercker/stern
[logrus]: https://github.com/sirupsen/logrus
[slog]: https://github.com/slog-rs/slog
[less]: https://www.gnu.org/software/less/
[code of conduct]: ./CODE_OF_CONDUCT.md
[dco]: https://developercertificate.org/
[design]: ./doc/design/design.md
