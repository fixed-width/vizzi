#!/usr/bin/env python3
"""Transparent timing peer for native LSP experiments; never records source bodies."""
import json
import os
from pathlib import Path
import subprocess
import sys
import threading
import time

output = Path(os.environ['VIZZI_LSP_PROFILE'])
output.mkdir(parents=True, exist_ok=True)
start = time.monotonic_ns()
log = (output / 'protocol.jsonl').open('a')
lock = threading.Lock()
patch = json.loads(os.environ.get('VIZZI_LSP_CONFIG_PATCH', '{}'))
config_requests = set()

def merge(value, changes):
    for key, item in changes.items():
        if isinstance(item, dict):
            merge(value.setdefault(key, {}), item)
        else:
            value[key] = item

def record(direction, message=None, size=0):
    item = {'elapsedMs': (time.monotonic_ns()-start)/1e6,
            'wallNs': time.time_ns(), 'direction': direction, 'bytes': size}
    if message is not None:
        for key in ['id', 'method']:
            if key in message:
                item[key] = message[key]
        if message.get('method') == 'experimental/serverStatus':
            item['status'] = message.get('params')
        if message.get('method') == 'initialize':
            item['initializationOptions'] = message.get('params', {}).get('initializationOptions')
        if 'error' in message:
            item['error'] = message['error']
        if 'resources' in message:
            item['resources'] = message['resources']
    with lock:
        log.write(json.dumps(item)+'\n')
        log.flush()

def relay(source, destination, direction):
    try:
        while True:
            headers = []
            count = None
            while True:
                line = source.readline(8193)
                if not line:
                    return
                headers.append(line)
                if sum(map(len, headers)) > 8192:
                    raise ValueError('Header too large')
                if line in [b'\r\n', b'\n']:
                    break
                key, value = line.decode().split(':', 1)
                if key.lower() == 'content-length':
                    count = int(value)
            if count is None or count > 16*1024*1024:
                raise ValueError('Invalid frame length')
            body = source.read(count)
            if len(body) != count:
                return
            message = json.loads(body)
            changed = False
            if direction == 'server' and message.get('method') == 'workspace/configuration':
                config_requests.add(message.get('id'))
            if patch and direction == 'client':
                if message.get('method') == 'initialize':
                    merge(message['params']['initializationOptions'], patch)
                    changed = True
                elif message.get('id') in config_requests and isinstance(message.get('result'), list):
                    for config in message['result']:
                        if isinstance(config, dict):
                            merge(config, patch)
                    changed = True
            record(direction, message, count)
            if changed:
                body = json.dumps(message).encode()
                headers = [('Content-Length: %d\r\n\r\n' % len(body)).encode()]
            destination.write(b''.join(headers)+body)
            destination.flush()
    except (BrokenPipeError, OSError):
        pass
    except Exception as error:
        record('relay_error', {'error': repr(error)})

record('wrapper_start')
stderr = (output / 'server.stderr').open('wb')
server = subprocess.Popen([os.environ.get('VIZZI_REAL_RUST_ANALYZER', 'rust-analyzer')],
                          stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=stderr)
record('server_spawned')
def resources():
    while server.poll() is None:
        try:
            values = Path('/proc/%d/status' % server.pid).read_text().splitlines()
            selected = {line.split(':', 1)[0]: line.split(':', 1)[1].strip() for line in values if line.startswith(('VmRSS:', 'Threads:'))}
            record('resources', {'resources': selected})
        except OSError:
            break
        time.sleep(.1)
threading.Thread(target=resources, daemon=True).start()
threading.Thread(target=relay, args=(sys.stdin.buffer, server.stdin, 'client'), daemon=True).start()
try:
    relay(server.stdout, sys.stdout.buffer, 'server')
finally:
    server.kill()
    server.wait()
    record('wrapper_exit')
    stderr.close()
