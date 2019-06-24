#/usr/bin/env python3
# (C) Copyright 2019 Hewlett Packard Enterprise Development LP

import datetime
import fcntl
import json
import os
import sys
import time

from collections import OrderedDict
from inspect import currentframe, getframeinfo


def format_json(doc):
  doc['time'] = doc['time'].isoformat()

  return json.dumps(doc)


def format_logrus(doc):
  doc['time'] = doc['time'].isoformat()

  return ' '.join([
    '{}={}'.format(k, json.dumps(v))
    for k, v in doc.items()
  ])


def format_klog(doc):
  return '{level}{timestamp:%m%d %H:%M:%S.%f}{threadid:>8} {file}] {message}'.format(
    level=doc['level'][0].upper(),
    timestamp=doc['time'],
    threadid=doc['threadid'],
    file=doc['file'],
    message=doc['message']
  )


def log(message, level, formatter=format_json, **kwargs):
  frame = getframeinfo(currentframe().f_back.f_back)

  doc = OrderedDict()
  doc['time'] = datetime.datetime.now()
  doc['level'] = level
  doc['threadid'] = 1
  doc['file'] = '{}:{}'.format(frame.filename, frame.lineno)
  doc['message'] = message
  doc.update(kwargs)

  print(formatter(doc), flush=True)


def info(message, formatter=format_json, **kwargs):
  log(message, 'info', formatter, **kwargs)


def warning(message, formatter=format_json, **kwargs):
  log(message, 'warning', formatter, **kwargs)


def error(message, formatter=format_json, **kwargs):
  log(message, 'error', formatter, **kwargs)


def plain(*args, **kwargs):
  print(*args, **kwargs, flush=True)


def main():
  warning('klog test', formatter=format_klog)
  plain('plaintext messages')

  info('hello world')
  time.sleep(1.0)
  warning('this message has some metadata', foo='bar')

  error('this is a logrus-like message', formatter=format_logrus)


if __name__ == '__main__':
  main()
