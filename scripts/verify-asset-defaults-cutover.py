#!/usr/bin/env python3
"""Private-home defaults bootstrap and legacy asset CLI/MCP acceptance, with input off."""
import argparse
import base64
import copy
import hashlib
import json
import os
from pathlib import Path
import plistlib
import select
import shutil
import subprocess
import tempfile
import time
import uuid


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('cli', 'cli-bridge', 'host', 'mcp', 'bridge', 'output-dir'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    out = args.output_dir.resolve(); out.mkdir(parents=True, exist_ok=True)
    repo = Path(__file__).resolve().parents[1]
    with tempfile.TemporaryDirectory(prefix='tad-', dir='/private/tmp') as folder:
        root = Path(folder); binary = root/'bin'; binary.mkdir(mode=0o700)
        for source, name in [(args.cli,'thumble'),(args.cli_bridge,'thumble-cli-bridge'),(args.host,'thumble-host'),(args.mcp,'thumble-mcp'),(args.bridge,'thumble-bridge')]:
            shutil.copy2(source.resolve(),binary/name)
        env = dict(os.environ, PATH=str(binary)+os.pathsep+os.environ.get('PATH',''))
        for name in ('THUMBLE_HOST_STATE_DIR','THUMBLE_HOST_CONTROL_SOCKET','POCKETPAD_HOST_STATE_DIR','POCKETPAD_HOST_CONTROL_SOCKET'):
            env.pop(name,None)
        fixture = json.loads((repo/'Host/fixtures/owned-configuration/v1/document.json').read_bytes())
        profile = copy.deepcopy(fixture['profiles'][1]); profile.pop('profileFormatVersion',None); profile['orientationPreference']='landscape'
        for slot in ('landscapeCustomization','portraitCustomization'):
            profile[slot]=copy.deepcopy(profile['customization'])
        selected=profile['id']; profile['futureInteger']=9007199254740993
        profile['launchTarget']=dict(kind='application',displayName='WoW fixture',filePath='/Applications/Fixture.app',attachedAt=1)
        token='private-defaults-fixture-token'
        envelope=dict(profiles=[profile],activeProfileID=selected,defaultProfileID=selected)
        fields={'PocketPad.gamepadConfigurationProfiles.v1':json.dumps(envelope).encode(),'PocketPadMac.serverIdentity.v1':'preserved-defaults-server','PocketPadMac.trustedClients.v1':json.dumps([dict(token=token,clientName='Fixture phone',createdAt=1,lastSeenAt=2)]).encode()}
        source=root/'reviewed.plist';source.write_bytes(plistlib.dumps(fields));original=source.read_bytes();digest=hashlib.sha256(original).hexdigest()
        pixel=root/'pixel.png';pixel.write_bytes(base64.b64decode('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Wl6X1cAAAAASUVORK5CYII='))
        sequence=0; mcp=None; host=None
        def command(program,*words,allowed=True):
            result=subprocess.run([str(binary/program),*words],env=env,capture_output=True,text=True,timeout=90)
            assert (result.returncode==0)==allowed,(program,words,result.stderr[-2000:]);return result.stdout
        def rpc(method,params):
            nonlocal sequence
            sequence+=1;mcp.stdin.write((json.dumps(dict(jsonrpc='2.0',id=sequence,method=method,params=params))+'\n').encode());mcp.stdin.flush()
            ready,_,_=select.select([mcp.stdout],[],[],60);assert ready,'MCP timeout'
            result=json.loads(mcp.stdout.readline());assert result['id']==sequence and 'error' not in result,result
            return result['result']
        def call(name,arguments,allowed=True):
            result=rpc('tools/call',dict(name=name,arguments=arguments));assert bool(result.get('isError'))!=allowed,result
            if not allowed:return result
            value=result.get('structuredContent') or json.loads(next(row['text'] for row in result['content'] if row['type']=='text'))
            assert token not in json.dumps(value)
            return value
        def initialize():
            rpc('initialize',dict(protocolVersion='2024-11-05',capabilities={},clientInfo=dict(name='asset-defaults-cutover',version='1')))
            mcp.stdin.write(b'{"jsonrpc":"2.0","method":"notifications/initialized"}\n');mcp.stdin.flush()
        with (out/'mcp.log').open('w') as mlog,(out/'receiver.log').open('w') as hlog:
            try:
                for method in ('cli','mcp'):
                    home=root/method;home.mkdir(mode=0o700);env['HOME']=str(home)
                    authority=home/'Library/Application Support/ThumbleHost';file=authority/'state.json'
                    preferences=home/'Library/Preferences/com.codybontecou.PocketPadMac.plist';preferences.parent.mkdir(parents=True);preferences.write_bytes(b'invalid prefs must remain unread and untouched')
                    if method=='cli':
                        command('thumble-host','configuration','import-desktop-defaults','--source-file',str(source),'--source-sha256','0'*64,allowed=False);assert not file.exists()
                        receipt=json.loads(command('thumble-host','configuration','import-desktop-defaults','--source-file',str(source),'--source-sha256',digest))
                    else:
                        mcp=subprocess.Popen([str(binary/'thumble-mcp'),'--allow-config-write'],env=env,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=mlog);initialize()
                        receipt=call('import_desktop_defaults',dict(sourceFile=str(source),sourceSHA256=digest))
                    assert receipt['trustedClientCount']==1 and receipt['profileCount']==1 and not receipt['replayed']
                    state=json.loads(file.read_bytes());assert state['serverID']=='preserved-defaults-server' and token in state['trustedClients'];assert state['profiles'][0]['launchTarget']==profile['launchTarget']
                    assert (authority/'desktop-defaults-source.plist').read_bytes()==original and (authority/'desktop-defaults-source.plist').stat().st_mode & 0o777==0o600
                    saved=file.read_bytes()
                    if method=='cli':replay=json.loads(command('thumble-host','configuration','import-desktop-defaults','--source-file',str(source),'--source-sha256',digest))
                    else:replay=call('import_desktop_defaults',dict(sourceFile=str(source),sourceSHA256=digest));mcp.terminate();mcp.wait(timeout=10);mcp=None
                    assert replay['replayed'] and file.read_bytes()==saved
                    invocation=str(uuid.uuid4());words=('asset','import',str(pixel),'--name','Fixture','--invocation-id',invocation)
                    command('thumble',*words);saved=file.read_bytes();command('thumble',*words);assert file.read_bytes()==saved
                    state=json.loads(saved);asset=state['profiles'][0]['customization']['assetLibrary']['assets'][-1]['id']
                    command('thumble','asset','rename',asset,'Renamed','--role','texture')
                    state=json.loads(file.read_bytes());p=state['profiles'][0]
                    for slot in ('customization','landscapeCustomization','portraitCustomization'):
                        entry=next(v for v in p[slot]['assetLibrary']['assets'] if v['id']==asset);assert entry['name']=='Renamed' and entry['role']=='texture' and base64.b64decode(entry['data'])==pixel.read_bytes()
                    assert p['futureInteger']==9007199254740993 and p['launchTarget']==profile['launchTarget']
                    command('thumble','asset','show',asset);command('thumble','layout','preview','--native','--profile',selected,'-o',str(out/(method+'-preview.png')))
                    host=subprocess.Popen([str(binary/'thumble-host'),'run','--port','0','--no-bonjour','--no-input','--allow-config-write'],env=env,stdout=hlog,stderr=hlog)
                    deadline=time.monotonic()+20
                    while not (authority/'control.sock').exists():assert host.poll() is None and time.monotonic()<deadline;time.sleep(.05)
                    mcp=subprocess.Popen([str(binary/'thumble-mcp'),'--allow-config-write'],env=env,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=mlog);initialize()
                    call('import_desktop_defaults',dict(sourceFile=str(source),sourceSHA256=digest),allowed=False)
                    revision=json.loads(file.read_bytes())['configurationRevision']
                    result=call('edit_profile_asset',dict(profileID=selected,expectedConfigurationRevision=revision,invocationID=str(uuid.uuid4()),edit=dict(action='set',assetID=asset,name='MCP name')))
                    assert result
                    command('thumble','asset','remove',asset)
                    mcp.terminate();mcp.wait(timeout=10);mcp=None;host.terminate();host.wait(timeout=10);host=None
                    assert source.read_bytes()==original and preferences.read_bytes()==b'invalid prefs must remain unread and untouched'
                # Fresh startup is independent of malformed desktop preferences.
                home=root/'fresh';preferences=home/'Library/Preferences/com.codybontecou.PocketPadMac.plist';preferences.parent.mkdir(parents=True);preferences.write_bytes(b'bad plist');env['HOME']=str(home)
                command('thumble','server','start','--port','0','--no-bonjour','--no-input')
                command('thumble','server','stop');assert preferences.read_bytes()==b'bad plist'
            finally:
                for process in (mcp,host):
                    if process is not None and process.poll() is None:process.terminate();process.wait(timeout=10)
        result=dict(cliAndMcpDefaultsBootstrap=True,pairingAndIdentityPreserved=True,localLaunchPreserved=True,exactPrivateBackup=True,sourceChecksumAndReplay=True,runningAndExistingAuthorityProtected=True,legacyAssetThreeSlots=True,helperFreeCommitReplay=True,metadataBytesAndIntegersPreserved=True,mcpLegacyAssetWrite=True,nativePreviews=True,freshStartupIgnoresDesktopPreferences=True,installedAuthorityTouched=False)
        (out/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
if __name__=='__main__':main()
