#!/usr/bin/env python3
"""Verify installed CLI/MCP addon onboarding with no receiver or desktop GUI."""
import argparse
import json
import os
from pathlib import Path
import select
import subprocess
import tempfile
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--bin', type=Path, required=True)
args = parser.parse_args()
installed = args.bin.resolve()
addons = ['ConsolePort', 'ConsolePort_Bar', 'ConsolePort_Config', 'ConsolePort_Cursor',
          'ConsolePort_Keyboard', 'ConsolePort_Menu', 'ConsolePort_Rings', 'ConsolePort_Target', 'ConsolePort_World']
with tempfile.TemporaryDirectory(prefix='thumble-wow-setup-', dir='/private/tmp') as work:
    work = Path(work)
    home = work / 'home'
    home.mkdir(mode=0o700)
    client = work / '_classic_era_'
    client.mkdir()
    source = work / 'release'
    for name in addons:
        folder = source / name
        folder.mkdir(parents=True)
        version = '## Version: 3.3.11' if name == 'ConsolePort' else '## RequiredDeps: ConsolePort'
        (folder / (name + '.toc')).write_text('## Interface: 11509, 16001\n' + version + '\nMain.lua\n')
        (folder / 'Main.lua').write_text('-- isolated fixture\n')
    environment = dict(os.environ, HOME=str(home))
    def cli(*extra, valid=True):
        result = subprocess.run([str(installed / 'thumble'), 'wow', 'setup', '--client-root', str(client),
            '--consoleport-source', str(source), '--json', *extra], env=environment, capture_output=True, text=True, timeout=30)
        if not valid:
            assert result.returncode != 0, 'Invalid plan unexpectedly applied'
            return
        assert result.returncode == 0, result.stderr
        return json.loads(result.stdout)
    plan = cli()
    assert plan['changed'] and not plan['applied']
    assert len(plan['plan']['addons']) == 10 and not plan['plan']['conflicts']
    assert not (client / 'Interface').exists(), 'Planning must not write directories'
    cli('--apply', '--expected-plan-sha256', '0' * 64, valid=False)
    assert not (client / 'Interface').exists()
    request_id = 0
    with (work / 'mcp.log').open('w') as log:
        mcp = subprocess.Popen([str(installed / 'thumble-mcp'), '--control-socket', str(work / 'no-receiver.sock'), '--allow-config-write'],
            env=environment, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=log)
        def send(message):
            mcp.stdin.write((json.dumps(message) + '\n').encode())
            mcp.stdin.flush()
        def rpc(method, params):
            global request_id
            request_id += 1
            send(dict(jsonrpc='2.0', id=request_id, method=method, params=params))
            deadline = time.monotonic() + 20
            while time.monotonic() < deadline:
                ready, _, _ = select.select([mcp.stdout], [], [], max(0, deadline - time.monotonic()))
                if not ready:
                    break
                line = mcp.stdout.readline()
                assert line, 'MCP closed before its response'
                response = json.loads(line)
                if response.get('id') != request_id:
                    continue
                assert 'error' not in response, response.get('error')
                return response['result']
            raise AssertionError('MCP request timed out')
        def tool(arguments):
            result = rpc('tools/call', dict(name='setup_wow_consoleport', arguments=arguments))
            assert not result.get('isError'), result.get('content')
            if 'structuredContent' in result:
                return result['structuredContent']
            return json.loads(next(item['text'] for item in result['content'] if item['type'] == 'text'))
        try:
            rpc('initialize', dict(protocolVersion='2024-11-05', capabilities={}, clientInfo=dict(name='wow-setup-verifier', version='1')))
            send(dict(jsonrpc='2.0', method='notifications/initialized'))
            names = [tool['name'] for tool in rpc('tools/list', {})['tools']]
            assert 'setup_wow_consoleport' in names and 'wow_sync' in names
            request = dict(clientRoot=str(client), consolePortSource=str(source))
            mcp_plan = tool(request)
            assert mcp_plan == plan, 'CLI and MCP must produce the same exact plan'
            applied = tool(dict(request, apply=True, expectedPlanSHA256=plan['plan']['planSHA256']))
            assert applied['applied'] and applied['changed']
            backup = Path(applied['backupDirectory'])
            assert (backup / 'phase').read_text() == 'committed\n'
            assert (backup.stat().st_mode & 0o777) == 0o700
            link = client / 'Interface/AddOns/ThumbleLink'
            assert (link / 'ThumbleLinkOnboarding.lua').is_file()
            assert 'enabled = true' in (link / 'ThumbleLinkSetup.lua').read_text()
            unchanged = cli()
            assert not unchanged['changed'] and not unchanged['applied']
            repeated = cli('--apply', '--expected-plan-sha256', unchanged['plan']['planSHA256'])
            assert repeated['applied'] and not repeated['changed'] and 'backupDirectory' not in repeated
            assert not (home / 'Library/Application Support/ThumbleHost/state.json').exists(), 'Setup must not create a second profile catalog'
            assert not list(installed.glob('*.app')), 'Verifier expects a tools-only installation'
            print('PASS: public CLI/MCP plan parity, exact-plan apply, private receipt, noop repeat, no receiver/catalog/desktop GUI')
        finally:
            mcp.terminate()
            try:
                mcp.wait(timeout=5)
            except subprocess.TimeoutExpired:
                mcp.kill(); mcp.wait()
