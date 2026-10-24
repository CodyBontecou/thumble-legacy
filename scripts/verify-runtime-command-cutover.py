#!/usr/bin/env python3
"""Verify runtime actions in private authorities; never prompt real OS permissions."""
import argparse
import fcntl
import importlib.util
import json
import os
from pathlib import Path
import select
import shutil
import socket
import subprocess
import tempfile
import threading
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bin', required=True, type=Path)
    args = parser.parse_args()
    spec = importlib.util.spec_from_file_location('phone_wire_fixture', Path(__file__).with_name('verify-owned-phone-wire.py'))
    wire = importlib.util.module_from_spec(spec); spec.loader.exec_module(wire)
    with tempfile.TemporaryDirectory(prefix='trc-', dir='/private/tmp') as folder:
        root = Path(folder); binary = root/'bin'; binary.mkdir(mode=0o700)
        for name in ('thumble','thumble-cli-bridge','thumble-host','thumble-mcp'):
            shutil.copy2(args.bin.resolve()/name, binary/name)
        home = root/'home'; home.mkdir(mode=0o700)
        authority = home/'Library/Application Support/ThumbleHost'
        env = dict(os.environ, HOME=str(home), THUMBLE_HOST_STATE_DIR=str(authority), THUMBLE_HOST_CONTROL_SOCKET=str(authority/'control.sock'))
        def cli(*arguments, ok=True):
            result = subprocess.run([str(binary/'thumble'),*arguments],env=env,capture_output=True,text=True,timeout=15)
            assert (result.returncode==0)==ok,(arguments,result.stderr)
            return result.stdout
        def host_status():
            return json.loads(subprocess.check_output([str(binary/'thumble-host'),'status','--json'],env=env,text=True,timeout=10))
        def mcp(tool, arguments=None, write=False, ok=True):
            process = subprocess.Popen([str(binary/'thumble-mcp'),*(['--allow-config-write'] if write else [])],env=env,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.DEVNULL,text=True)
            def exchange(message):
                process.stdin.write(json.dumps(message)+'\n');process.stdin.flush()
                while True:
                    assert select.select([process.stdout],[],[],10)[0],'MCP response timeout'
                    line=process.stdout.readline();assert line,'MCP exited'
                    value=json.loads(line)
                    if value.get('id')==message.get('id'):return value
            try:
                exchange(dict(jsonrpc='2.0',id=1,method='initialize',params=dict(protocolVersion='2024-11-05',capabilities={},clientInfo=dict(name='runtime-fixture',version='1'))))
                process.stdin.write(json.dumps(dict(jsonrpc='2.0',method='notifications/initialized'))+'\n');process.stdin.flush()
                reply=exchange(dict(jsonrpc='2.0',id=2,method='tools/call',params=dict(name=tool,arguments=arguments or {})))
                result=reply.get('result',{});failed='error' in reply or result.get('isError',False)
                assert failed!=ok,reply
                return json.loads(result['content'][0]['text']) if ok else reply
            finally:
                process.stdin.close()
                try:process.wait(timeout=5)
                except subprocess.TimeoutExpired:process.terminate();process.wait(timeout=5)
        for arguments in [('pairing','cancel'),('accessibility','status'),('accessibility','prompt'),('accessibility','open'),('profile','show','--legacy-desktop','--json'),('monitor','--path')]:
            cli(*arguments,ok=False)
        assert not authority.exists(),'Stopped commands initialized authority'
        authority.mkdir(parents=True,mode=0o700)
        calls=[];errors=[];missing=[False];done=threading.Event()
        listener=socket.socket(socket.AF_UNIX);listener.bind(str(authority/'control.sock'));os.chmod(authority/'control.sock',0o600);listener.listen();listener.settimeout(.1)
        lease=(authority/'runtime.lock').open('a+');fcntl.flock(lease,fcntl.LOCK_EX|fcntl.LOCK_NB)
        def fake_receiver():
            while not done.is_set():
                try:connection,_=listener.accept()
                except socket.timeout:continue
                try:
                    with connection:
                        connection.settimeout(5);data=b''
                        while b'\n' not in data:data+=connection.recv(8192)
                        request=json.loads(data);calls.append(request)
                        result=dict(ok=True)
                        if not missing[0]:
                            if request['command']=='accessibility':result['accessibilityTrusted']=False
                            elif request['command']=='cancel-pairing':result['pairingCancelled']=False
                            else:raise AssertionError(request)
                        connection.sendall(json.dumps(result).encode()+b'\n')
                except Exception as error:errors.append(repr(error))
        worker=threading.Thread(target=fake_receiver);worker.start()
        try:
            for action in ('status','refresh','prompt','request','open','settings'):
                reply=json.loads(cli('accessibility',action,'--json'))
                assert reply['response']['accessibilityTrusted'] is False
            assert cli('pairing','cancel').strip()=='No pending pairing request.'
            count=len(calls)
            for action in ('prompt','open'):mcp('accessibility_action',dict(action=action),ok=False)
            assert len(calls)==count,'Missing opt-in reached receiver'
            for action in ('prompt','open'):
                assert mcp('accessibility_action',dict(action=action),write=True)['trusted'] is False
            assert mcp('accessibility_status')['trusted'] is False
            assert mcp('pairing_cancel')['cancelled'] is False
            missing[0]=True
            cli('accessibility','status',ok=False);cli('pairing','cancel',ok=False)
            mcp('accessibility_status',ok=False);mcp('pairing_cancel',ok=False)
            assert not errors,errors
        finally:
            done.set();worker.join(timeout=5);assert not worker.is_alive()
            listener.close();(authority/'control.sock').unlink();fcntl.flock(lease,fcntl.LOCK_UN);lease.close()
        assert not (authority/'state.json').exists(),'Runtime operations seeded configuration'
        with (root/'receiver.log').open('w') as log:
            process=subprocess.Popen([str(binary/'thumble-host'),'run','--port','0','--no-bonjour','--no-input'],env=env,stdout=log,stderr=log)
            peers=[]
            try:
                deadline=time.monotonic()+15
                while not (authority/'control.sock').exists():
                    assert process.poll() is None and time.monotonic()<deadline;time.sleep(.05)
                status=host_status();port=status['port'];before=(authority/'state.json').read_bytes()
                payload=mcp('pairing_payload');local=json.loads(cli('pairing','payload'))
                assert payload['serverID']==local['serverID']==status['serverID']
                assert payload['pairingCode']==local['pairingCode'] and set(payload['urls'])==set(local['urls'])
                for use_mcp in (False,True):
                    peer=wire.WebSocket(port);peers.append(peer)
                    peer.send(dict(type='pairing_request',timestamp=0,clientName='Private runtime fixture',supportedProfileFormats=[4]))
                    peer.until('pairing_challenge')
                    code=json.loads(cli('pairing','payload'))['pairingCode']
                    assert host_status()['core']['pairingPending']
                    if use_mcp:assert mcp('pairing_cancel')['cancelled'] is True
                    else:assert cli('pairing','cancel').strip()=='Cancelled pending pairing.'
                    assert not host_status()['core']['pairingPending']
                    assert json.loads(cli('pairing','payload'))['pairingCode']!=code
                    stale=wire.WebSocket(port);peers.append(stale)
                    stale.send(dict(type='hello',timestamp=0,pairingCode=code,clientName='Cancelled fixture',supportedProfileFormats=[4]))
                    assert stale.until('error')['message']=='Wrong pairing code'
                    assert (authority/'state.json').read_bytes()==before
                assert mcp('pairing_cancel')['cancelled'] is False
                permission=json.loads(cli('accessibility','status','--json'))['response']['accessibilityTrusted']
                assert mcp('accessibility_status')['trusted']==permission
            finally:
                for peer in peers:peer.socket.close()
                process.terminate();process.wait(timeout=10)
        print(json.dumps(dict(headlessPermissionAliases=True,systemActionsMockedOnly=True,mcpSystemActionOptIn=True,missingReceiptsRefused=True,stoppedAuthorityUntouched=True,cliAndMcpPendingCancellation=True,oldChallengeRejected=True,pairingPayloadParity=True,trustAndConfigurationUnchanged=True,desktopSnapshotRetired=True,installedAuthorityTouched=False)))


if __name__=='__main__':main()
