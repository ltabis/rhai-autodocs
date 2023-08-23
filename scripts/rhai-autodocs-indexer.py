#!/usr/bin/env python3
import sys

if len(sys.argv) != 2:
    print("""usage: rhai-autodocs-indexer.py <path-to-source-file>

    This script searches for any `# rhai-autodocs:index:` comment and
    automatically adds the index number following the order of the function
    in the source file.

    See the `FunctionOrder::ByIndex` option from the rhai-autodocs crate.
""")
    exit(1)

file = sys.argv[1]

oldfile = open(file, 'r')
newfile = open(f"{file}.autodocs", 'w')

function_order = 1
for line in oldfile:
    newline = ""
    if "# rhai-autodocs:index:" in line:
        newline = f"    /// # rhai-autodocs:index:{function_order}\n"
        function_order += 1
    else:
        newline = line

    newfile.write(newline)
