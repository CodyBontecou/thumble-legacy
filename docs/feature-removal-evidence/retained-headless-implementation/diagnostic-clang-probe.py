#!/usr/bin/python3
import os
import sys
compiler = '/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/bin/clang'
args = sys.argv[1:]
# Xcode's macro probe does not require verbose driver diagnostics. Keeping its
# stdout identical avoids the observed stderr-pipe deadlock in SWBBuildService.
if all(flag in args for flag in ('-v', '-E', '-dM', '/dev/null')):
    args.remove('-v')
os.execv(compiler, [compiler, *args])
