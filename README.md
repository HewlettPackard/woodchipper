# woodchipper

[![CircleCI](https://circleci.com/gh/HewlettPackard/woodchipper.svg?style=svg)](https://circleci.com/gh/HewlettPackard/woodchipper)

Process your logs into 

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

 3. For best use with Kubernetes, install the
    [`kubectl` plugin](./kubectl-woodchipper) and watch some pods:

    ```bash
    kubectl woodchipper -n my-namespace app=my-app
    ```

## Similar Projects

 * [stern] has similar Kubernetes tailing features
 * [logrus] has built-in pretty printing when a TTY is attached
 * [slog] provides structured pretty printing
 * [less] supports paging, searching, and input following

[releases]: https://github.com/HewlettPackard/woodchipper/releases/latest
[stern]: https://github.com/wercker/stern
[logrus]: https://github.com/sirupsen/logrus
[slog]: https://github.com/slog-rs/slog
[less]: https://www.gnu.org/software/less/
