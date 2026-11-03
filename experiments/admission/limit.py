#!/usr/bin/env python3
"""Apply a child-only virtual address-space limit, then replace this process."""
import os
import resource
import sys

limit = int(sys.argv[1]) * 1024 * 1024
resource.setrlimit(resource.RLIMIT_AS, (limit, limit))
os.execv(sys.argv[2], sys.argv[2:])
