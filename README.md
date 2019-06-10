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

 3. For easier use with Kubernetes, install the [`kubectl` plugin][plugin]
    wrapper script and watch some pods:

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

### `kubectl` plugin (for `kubectl` 1.13+)

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

## Similar Projects

 * [stern] has similar Kubernetes tailing features
 * [logrus] has built-in pretty printing when a TTY is attached
 * [slog] provides structured pretty printing
 * [less] supports paging, searching, and input following

[plugin]: ./kubectl-woodchipper
[releases]: https://github.com/HewlettPackard/woodchipper/releases/latest
[stern]: https://github.com/wercker/stern
[logrus]: https://github.com/sirupsen/logrus
[slog]: https://github.com/slog-rs/slog
[less]: https://www.gnu.org/software/less/
