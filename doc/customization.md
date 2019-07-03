# Customization

woodchipper can be customized using command-line flags and environment
variables.

A few of the more complex options are discussed here, but for a full list of
options, refer to `woodchipper --help`.

## Color Schemes

woodchipper can use any [base16 color scheme][base16]. To use:

 * Save a scheme's `.yaml` configuration somewhere local, e.g.
   [`classic-dark.yaml`][classic-dark]
 * Run woodchipper with `--style=base16:path/to/classic-dark.yaml`
 * Once satisfied with results, set:

   ```
   export WD_STYLE=base16:path/to/classic-dark.yaml
   ```

   ...in your environment.

[base16]: https://github.com/chriskempson/base16#scheme-repositories
[classic-dark]: https://github.com/detly/base16-classic-scheme/blob/master/classic-dark.yaml

## Log Formats

In addition to the built-in formats, woodchipper supports custom regex-based
parsers. These can be used to support many application-specific log formats that
don't require a more advanced parser.

To add a custom log format, create a `.yaml` file containing a list of regexes:

```yaml
- pattern: ...
  datetime: ...

- pattern: ...
  datetime: ...
  datetime_prepend: ...
```

Each `pattern` field should contain a regex with various
[named capture groups][groups]:
 * `(?P<datetime>...)`

   Captures the datetime string - this is further parsed later using the format
   set in the `datetime` field.
 * `(?P<level>...)`

   Captures the log level (`I`, `INFO`, etc; case insensitive)
 * `(?P<text>...)`

   Captures the main message text.

Any additional named capture groups will be added as message metadata. Certain
classifiers may have special display-time rules for metadata fields; for
example, the `file` or `caller` fields will be shown as right-aligned context
if there's enough available screen width.

The `datetime` field contains parsing rules for the captured `datetime` field.
It has two built-in formats, `rfc2822` and `rfc3339`, but a free-form
[chrono `stftime`][strftime] string can be set here as well.

Note that chrono requires fully-formed datetime strings, and won't fill in
missing fields for you. If your log format omits some fields (e.g. `klog`
doesn't output the year), you can use `datetime_prepend` to add missing fields
to the incoming datetime string based on the current UTC time. This field should
contain another strftime format string with only the missing fields from the
original input.

Finally, to make use of the regex config, first test with:

```
woodchipper --regexes path/to/regexes.yaml
```

... and once satisfied with the results, add:

```bash
export WD_REGEXES=path/to/regexes.yaml
```

... to your environment.

[groups]: https://docs.rs/regex/1.1.7/regex/#grouping-and-flags
[strftime]: https://docs.rs/chrono/0.4.7/chrono/format/strftime/index.html

### Example

As an example, take this Python logging example, `logs.py`:

```python
import logging

logging.basicConfig(
    format='%(asctime)-15s - %(levelname)-8s - %(filename)s:%(lineno)d - %(message)s',
    level='DEBUG'
)

logger = logging.getLogger('test')
logger.debug('this is a debug message')
logger.info('this is an info message')
logger.warning('this is a warning message')
logger.error('this is an error message')
```

It produces log messages like this:
```
$ python3 -u logs.py 2>&1
2019-07-03 12:02:13,977 - DEBUG    - test.py:9 - this is a debug message
2019-07-03 12:02:13,977 - INFO     - test.py:10 - this is an info message
2019-07-03 12:02:13,977 - WARNING  - test.py:11 - this is a warning message
2019-07-03 12:02:13,977 - ERROR    - test.py:12 - this is an error message
```

Create a YAML file, e.g. `~/.woodchipper-regexes.yaml` with the following
content:

```yaml
- pattern: |-
    ^(?P<datetime>[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2})(?:,[0-9]+) - (?P<level>\w+)\s* - (?P<file>\S+)\s* -(?P<text>.+)$
  datetime: '%Y-%m-%d %H:%M:%S'
```

Now pipe it through `woodchipper` with the `--regexes` flag set to point to your
YAML file:

```
$ python3 -u test.py 2>&1 | woodchipper -r json --regexes ~/.woodchipper-regexes.yaml
{"kind":"regex","timestamp":"2019-07-03T12:07:15Z","level":"debug","text":" this is a debug message","metadata":{"file":"test.py:9"}}
{"kind":"regex","timestamp":"2019-07-03T12:07:15Z","level":"info","text":" this is an info message","metadata":{"file":"test.py:10"}}
{"kind":"regex","timestamp":"2019-07-03T12:07:15Z","level":"warning","text":" this is a warning message","metadata":{"file":"test.py:11"}}
{"kind":"regex","timestamp":"2019-07-03T12:07:15Z","level":"error","text":" this is an error message","metadata":{"file":"test.py:12"}}
```

The three primary fields (timestamp, level, text) were captured, along with an
additional metadata field containing the `file`.

Note that chrono's strftime doesn't seem to support custom millisecond separator
characters if they aren't right-aligned to a particular width, and python's
`%(asctime)` seems to like using commas. The example pattern above just excludes
the milliseconds as they won't be displayed anyway.

Finally, `WD_REGEXES` may be set in your environment to make use of this regex
configuration without needing to manually pass in `--regexes`.
