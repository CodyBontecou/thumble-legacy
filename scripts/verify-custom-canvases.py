#!/usr/bin/env python3
"""Exercise custom canvas writes through actual local CLI/MCP authority."""
import argparse, hashlib, json, os, select, shutil, subprocess, tempfile, time, uuid
from pathlib import Path


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for key in ('cli','bridge','cli-bridge','host','mcp','output-dir'):parser.add_argument('--'+key,type=Path,required=True)
    args=parser.parse_args();args.output_dir.mkdir(parents=True,exist_ok=True)
    repo=Path(__file__).resolve().parent.parent
    fixture=json.loads((repo/'Host/fixtures/owned-configuration/v1/document.json').read_bytes())
    with tempfile.TemporaryDirectory(prefix='thumble-custom-canvas-',dir='/private/tmp') as directory:
        root=Path(directory);install=root/'bin';install.mkdir(mode=0o700)
        for p,name in [(args.cli,'thumble'),(args.bridge,'native-helper'),(args.cli_bridge,'thumble-cli-bridge'),(args.host,'thumble-host'),(args.mcp,'thumble-mcp')]:shutil.copy2(p,install/name)
        wrapper=install/'thumble-bridge'
        wrapper.write_text('#!/usr/bin/python3\nimport subprocess,sys\nfrom pathlib import Path\nr=Path(__file__).parent\ndata=sys.stdin.buffer.read()\np=subprocess.run([str(r/"native-helper")],input=data,capture_output=True)\nout=Path('+repr(str(args.output_dir.resolve()))+')\n(out/"last-native-request.json").write_bytes(data)\n(out/"last-native-response.json").write_bytes(p.stdout)\n(out/"last-native-stderr.txt").write_bytes(p.stderr)\nsys.stdout.buffer.write(p.stdout)\nsys.stderr.buffer.write(p.stderr)\nsys.exit(p.returncode)\n');wrapper.chmod(0o700)
        env=dict(os.environ,PATH=str(install)+os.pathsep+os.environ.get('PATH',''))
        for key in ('THUMBLE_HOST_STATE_DIR','THUMBLE_HOST_CONTROL_SOCKET','POCKETPAD_HOST_STATE_DIR','POCKETPAD_HOST_CONTROL_SOCKET'):env.pop(key,None)
        def cli(*args,allowed=True):
            p=subprocess.run([str(install/'thumble'),*args],env=env,capture_output=True,text=True,timeout=60)
            assert (p.returncode==0)==allowed,(args,p.stderr[-2500:]);return p.stdout
        for owned in (False,True):
            home=root/('owned' if owned else 'legacy');authority=home/'Library/Application Support/ThumbleHost';authority.mkdir(parents=True);env['HOME']=str(home)
            profiles=json.loads(json.dumps(fixture['profiles']))
            if not owned:
                # A classic profile without resources remains a supported legacy authority.
                profiles=[profiles[1]];profiles[0].pop('profileFormatVersion',None);profiles[0]['orientationPreference']='landscape'
            selected=profiles[0]['id']
            profiles[0]['futureExact']='EXACT_FRACTION'
            state=dict(schemaVersion=4 if owned else 3,serverID='private-custom-canvas',trustedClients={},configurationRevision=41,configurationUpdatedAt=100,recentConfigurationCommits=[],profiles=profiles,activeProfileID=selected,defaultProfileID=selected,keyBindings={},outputBindings={},profileKeyBindings={},profileOutputBindings={})
            file=authority/'state.json';file.write_text(json.dumps(state).replace('"EXACT_FRACTION"','0.12345678901234567890123456789'));file.chmod(0o600)
            def current():return json.loads(file.read_bytes())
            before=current();nonce=str(uuid.uuid4());command=('device','set','custom','--size','999x777','--orientation','landscape','--profile',selected,'--invocation-id',nonce)
            cli(*command);after=current();assert after['configurationRevision']==42
            assert after['profiles'][0]['customization']['deviceCanvas']['frameID']=='custom-777x999-landscape'
            expected=before['profiles'][0];expected['customization']['deviceCanvas']['frameID']='custom-777x999-landscape';expected['updatedAt']=after['profiles'][0]['updatedAt']
            # Legacy native setters add the selected orientation mirror, which is
            # an internal compatibility representation absent from owned results.
            if not owned:expected['landscapeCustomization']=expected['customization']
            assert after['profiles'][0]==expected
            assert after['profiles'][1:]==before['profiles'][1:] and '0.12345678901234567890123456789' in file.read_text()
            saved=file.read_bytes();helper=install/'thumble-bridge';parked=install/'helper.parked';helper.rename(parked)
            try:cli(*command);assert file.read_bytes()==saved
            finally:parked.rename(helper)
            cli('device','set','custom','--size','999x777','--orientation','landscape','--profile',selected);assert current()['configurationRevision']==42
            shown=json.loads(cli('device','show','--profile',selected,'--json'));assert shown['id']=='custom-777x999-landscape'
            cli('customization','set','--device-size','1024x768','--orientation','landscape','--profile',selected);assert current()['configurationRevision']==43
            assert current()['profiles'][0]['customization']['deviceCanvas']['frameID']=='custom-768x1024-landscape'
            cli('customization','set','--device','custom','--device-width','1100','--device-height','700','--orientation','landscape','--profile',selected);assert current()['configurationRevision']==44
            assert current()['profiles'][0]['customization']['deviceCanvas']['frameID']=='custom-700x1100-landscape'
            if owned:
                saved=file.read_bytes();cli('device','set','custom','--size','390x844','--orientation','portrait','--profile',selected,allowed=False);assert file.read_bytes()==saved
                for scheme in ('light','dark'):
                    for action in ('base','left-trigger','right-trigger','both-triggers'):
                        image=args.output_dir/(scheme+'-'+action+'.png');cli('layout','preview','--profile',selected,'--native','--appearance',scheme,'--action-set',action,'-o',str(image),'--scale','1');assert image.read_bytes().startswith(bytes.fromhex('89504e470d0a1a0a'))
                host=mcp=None;sequence=0
                with (root/'host.log').open('w') as hlog,(root/'mcp.log').open('w') as mlog:
                    def rpc(method,params):
                        nonlocal sequence
                        sequence+=1;mcp.stdin.write((json.dumps(dict(jsonrpc='2.0',id=sequence,method=method,params=params))+'\n').encode());mcp.stdin.flush();deadline=time.monotonic()+60
                        while time.monotonic()<deadline:
                            ready,_,_=select.select([mcp.stdout],[],[],max(0,deadline-time.monotonic()));assert ready,'MCP timeout'
                            line=mcp.stdout.readline();assert line,'MCP closed';value=json.loads(line)
                            if value.get('id')==sequence:assert 'error' not in value,value;return value['result']
                        raise AssertionError('MCP timeout')
                    def call(name,params,allowed=True):
                        value=rpc('tools/call',dict(name=name,arguments=params));assert bool(value.get('isError'))!=allowed,value
                        if not allowed:return value
                        return value.get('structuredContent') or json.loads(next(v['text'] for v in value['content'] if v['type']=='text'))
                    try:
                        host=subprocess.Popen([str(install/'thumble-host'),'run','--port','0','--no-bonjour','--no-input','--allow-config-write'],env=env,stdout=hlog,stderr=hlog)
                        deadline=time.monotonic()+20
                        while not (authority/'control.sock').exists():assert host.poll() is None and time.monotonic()<deadline;time.sleep(.05)
                        mcp=subprocess.Popen([str(install/'thumble-mcp'),'--allow-config-write'],env=env,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=mlog)
                        rpc('initialize',dict(protocolVersion='2024-11-05',capabilities={},clientInfo=dict(name='custom-canvas-verifier',version='1')));mcp.stdin.write(b'{"jsonrpc":"2.0","method":"notifications/initialized"}\n');mcp.stdin.flush()
                        draft=call('begin_configuration_draft',dict(expectedConfigurationRevision=44));saved=file.read_bytes()
                        params=dict(draftId=draft['draftId'],expectedDraftRevision=draft['draftRevision'],operationId=str(uuid.uuid4()),operation=dict(type='device.set',profileID=selected,variant='landscape',frameID='custom-600x1200-landscape'))
                        edit=call('edit_configuration_draft',params);assert file.read_bytes()==saved
                        commit=dict(draftId=draft['draftId'],expectedDraftRevision=edit['draft']['draftRevision'],expectedConfigurationRevision=44,commitId=str(uuid.uuid4()))
                        assert call('save_configuration_draft',commit)['configurationRevision']==45
                        assert current()['profiles'][0]['customization']['deviceCanvas']['frameID']=='custom-600x1200-landscape'
                        saved=file.read_bytes();assert call('save_configuration_draft',commit)['idempotentReplay'] and file.read_bytes()==saved
                        bad=call('begin_configuration_draft',dict(expectedConfigurationRevision=45))
                        for invalid in ('custom-239x844-landscape','custom-390x1801-landscape','custom-844x390-landscape','custom-0390x844-landscape','custom-390x844-landscape/private','custom-390x844-portrait'):
                            call('edit_configuration_draft',dict(draftId=bad['draftId'],expectedDraftRevision=bad['draftRevision'],operationId=str(uuid.uuid4()),operation=dict(type='device.set',profileID=selected,variant='primary',frameID=invalid)),allowed=False);assert file.read_bytes()==saved
                    finally:
                        for process in (mcp,host):
                            if process is not None and process.poll() is None:process.terminate();process.wait(timeout=10)
            if owned:
                before_builtin=current()
                cli('device','set','iphone-16-pro-landscape','--profile',selected)
                assert current()['configurationRevision']==46
                assert current()['profiles'][0]['appearanceBank']==before_builtin['profiles'][0]['appearanceBank']
                assert current()['profiles'][0]['appearanceResources']==before_builtin['profiles'][0]['appearanceResources']
                cli('device','set','iphone-16-pro-landscape','--profile',selected);assert current()['configurationRevision']==46
                cli('layout','preview','--profile',selected,'--native','--appearance','dark','--action-set','both-triggers','-o',str(args.output_dir/'builtin-dark-both-triggers.png'),'--scale','1')
            (args.output_dir/('owned-state.json' if owned else 'legacy-state.json')).write_bytes(file.read_bytes())
        result=dict(classicAndOwnedCLI=True,customizationSizeAndDimensions=True,ownedEightStateAndResourcesPreserved=True,builtinPooledCanvasRegression=True,exactMetadataAndUnrelatedProfiles=True,helperFreeReplay=True,identicalSizeNoOp=True,ownedPortraitRefused=True,localMCPPrivateDraftCommitReplay=True,invalidWireIDsRefused=True,automaticMigrationActivated=False)
        (args.output_dir/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))


if __name__=='__main__':main()
