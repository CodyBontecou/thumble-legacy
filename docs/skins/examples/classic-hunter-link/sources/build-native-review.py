#!/usr/bin/env python3
"""Build review helper using the current CLI target's non-CLI source inventory.
Usage: python3 build-native-review.py CLI_SWIFT_FILE_LIST DEBUG_PRODUCTS OUTPUT_BINARY
Does not build app targets or modify source files. Requires existing ZIPFoundation products.
"""
import pathlib
import subprocess
import sys

if len(sys.argv) != 4:
    raise SystemExit(__doc__)
file_list, products, output = map(pathlib.Path, sys.argv[1:])
helper = pathlib.Path(__file__).resolve().with_name('RenderNativeImport.swift')
sources = [line.strip() for line in file_list.read_text().splitlines()
           if line.strip() and '/Sources/CLI/' not in line and '/Tests/' not in line]
if not sources or not any('/Sources/Shared/' in line for line in sources):
    raise SystemExit('Expected populated ThumbleCLI SwiftFileList with Shared source paths')
zip_object = products / 'ZIPFoundation.o'
if not zip_object.is_file():
    raise SystemExit(f'Missing built dependency: {zip_object}')
output.parent.mkdir(parents=True, exist_ok=True)
subprocess.run(['xcrun', 'swiftc', '-swift-version', '5', '-DDEBUG', '-parse-as-library',
                '-module-name', 'NativeImportReview', '-I', str(products), '-F', str(products),
                *sources, str(helper), str(zip_object), '-lz', '-o', str(output)], check=True)
