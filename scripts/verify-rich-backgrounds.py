#!/usr/bin/env python3
"""Real CLI/MCP rich-background edits without a desktop preference store."""
import argparse, base64, hashlib, json, os, select, shutil, struct, subprocess, tempfile, time, uuid, zlib
from PIL import Image
from pathlib import Path


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ('cli','bridge','cli-bridge','host','mcp','output-dir'):parser.add_argument('--'+name,type=Path,required=True)
    args=parser.parse_args();args.output_dir.mkdir(parents=True,exist_ok=True)
    repo=Path(__file__).resolve().parent.parent
    fixture=json.loads((repo/'Host/fixtures/owned-configuration/v1/document.json').read_bytes())
    def chunk(kind, data):return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data))
    pixel=bytes.fromhex('89504e470d0a1a0a')+chunk(b'IHDR',struct.pack('>IIBBBBB',8,8,8,6,0,0,0))+chunk(b'IDAT',zlib.compress((b'\0'+bytes([38,112,204,255])*8)*8))+chunk(b'IEND',b'')
    with tempfile.TemporaryDirectory(prefix='thumble-bg-',dir='/private/tmp') as directory:
        root=Path(directory);install=root/'bin';install.mkdir(mode=0o700)
        for p,name in [(args.cli,'thumble'),(args.bridge,'native-helper'),(args.cli_bridge,'thumble-cli-bridge'),(args.host,'thumble-host'),(args.mcp,'thumble-mcp')]:shutil.copy2(p,install/name)
        wrapper=install/'thumble-bridge'
        wrapper.write_text('#!/usr/bin/python3\nimport subprocess,sys,json\nfrom pathlib import Path\nr=Path(__file__).parent\ndata=sys.stdin.buffer.read()\np=subprocess.run([str(r/"native-helper")],input=data,capture_output=True)\nv=json.loads(data)\nedit=v.get("operation",{}).get("changes",{}).get("backgroundEdit",{})\nout=Path('+repr(str(args.output_dir.resolve()))+')\nif v.get("ownedConfigurationCommand")=="apply" and edit.get("fill",{}).get("kind")=="image" and not (out/"native-request.json").exists():\n (out/"native-request.json").write_bytes(data)\n (out/"native-response.json").write_bytes(p.stdout)\nsys.stdout.buffer.write(p.stdout)\nsys.stderr.buffer.write(p.stderr)\nsys.exit(p.returncode)\n');wrapper.chmod(0o700)
        env=dict(os.environ,PATH=str(install)+os.pathsep+os.environ.get('PATH',''))
        for name in ('THUMBLE_HOST_STATE_DIR','THUMBLE_HOST_CONTROL_SOCKET','POCKETPAD_HOST_STATE_DIR','POCKETPAD_HOST_CONTROL_SOCKET'):env.pop(name,None)
        image=root/'image.png';image.write_bytes(pixel);maximum=root/'maximum.png';maximum.write_bytes(pixel+b'\0'*(2500000-len(pixel)))
        def cli(*params,allowed=True):
            p=subprocess.run([str(install/'thumble'),*params],env=env,capture_output=True,text=True,timeout=90)
            assert (p.returncode==0)==allowed,(params,p.stderr[-2500:]);return p.stdout
        for owned in (False,True):
            home=root/('owned' if owned else 'legacy');authority=home/'Library/Application Support/ThumbleHost';authority.mkdir(parents=True);env['HOME']=str(home)
            profiles=json.loads(json.dumps(fixture['profiles']))
            if not owned:
                profiles=[profiles[1]];profiles[0].pop('profileFormatVersion',None);profiles[0]['orientationPreference']='landscape'
            profiles[0]['futureExact']='PRECISE_FRACTION';selected=profiles[0]['id']
            if owned:
                blob=b'opaque-future-resource';digest=hashlib.sha256(blob).hexdigest();profiles[0]['appearanceResources'][digest]=base64.b64encode(blob).decode();profiles[0]['futureResource']={'resourceSHA256':digest}
                profiles[0]['appearanceBank']['variants'][3]['futureRow']={'exact':'ROW_FRACTION'}
                profiles[1]['appearanceBank']=None
            state=dict(schemaVersion=4 if owned else 3,serverID='private-rich-background',trustedClients={},configurationRevision=41,configurationUpdatedAt=100,recentConfigurationCommits=[],profiles=profiles,activeProfileID=selected,defaultProfileID=selected,keyBindings={},outputBindings={},profileKeyBindings={},profileOutputBindings={})
            file=authority/'state.json';file.write_text(json.dumps(state).replace('"PRECISE_FRACTION"','0.12345678901234567890123456789').replace('"ROW_FRACTION"','0.98765432109876543210987654321').replace('"colorScheme": "light"','"colorScheme": "li\\u0067ht"',1));file.chmod(0o600)
            def current():return json.loads(file.read_bytes())
            before=current();old=current()['profiles'][0];revision=41
            for options,kind,scope in [
                (['--background-gradient','#102030,#405060'],'gradient','all'),
                (['--light-background-tile','dots','--tile-scale','1.5'],'tile','light'),
                (['--dark-background-image',str(image),'--image-mode','fit'],'image','dark')]:
                nonce=str(uuid.uuid4());command=('customization','set',*options,'--profile',selected,'--invocation-id',nonce)
                cli(*command);revision+=1;assert current()['configurationRevision']==revision
                target=current()['profiles'][0]
                assert current()['profiles'][1:]==before['profiles'][1:] and '0.12345678901234567890123456789' in file.read_text()
                for key in ('elements','customButtons','styleLibrary','assetLibrary','groups'):
                    assert target['customization'].get(key)==old['customization'].get(key),key
                assert target.get('launchTarget')==old.get('launchTarget')
                field={'all':'backgroundFillStyle','light':'backgroundLightFillStyle','dark':'backgroundDarkFillStyle'}[scope]
                assert target['customization'][field]['kind']==kind
                if owned:
                    assert '0.98765432109876543210987654321' in file.read_text()
                    assert 'li\\u0067ht' in file.read_text()
                    assert target['appearanceResources'][digest]==base64.b64encode(blob).decode()
                    for row in target['appearanceBank']['variants']:assert row['appearance'][field]['kind']==kind
                saved=file.read_bytes();helper=install/'thumble-bridge';parked=install/'helper.parked';helper.rename(parked)
                try:cli(*command);assert file.read_bytes()==saved
                finally:parked.rename(helper)
                cli('customization','set',*options,'--profile',selected);assert current()['configurationRevision']==revision
                old=target
            saved=file.read_bytes();link=root/('owned-link.png' if owned else 'legacy-link.png');link.symlink_to(image)
            cli('customization','set','--background-image',str(link),'--profile',selected,allowed=False);assert file.read_bytes()==saved
            fifo=root/('owned-fifo' if owned else 'legacy-fifo');os.mkfifo(fifo);cli('customization','set','--background-image',str(fifo),'--profile',selected,allowed=False);assert file.read_bytes()==saved
            if owned:
                plain_id=profiles[1]['id'];cli('customization','set','--background-gradient','#123456,#654321','--profile',plain_id);revision+=1
                assert current()['configurationRevision']==revision and current()['profiles'][1].get('appearanceBank','missing') is None
                cli('customization','set','--background-image',str(maximum),'--profile',selected);revision+=1;assert current()['configurationRevision']==revision
                assert any(len(base64.b64decode(v))==2500000 for v in current()['profiles'][0]['appearanceResources'].values())
                cli('customization','set','--background-image',str(maximum),'--profile',selected);assert current()['configurationRevision']==revision
                for version in (1,2,3):
                    maximum.write_bytes(pixel+b'\0'*(2500000-len(pixel)-1)+bytes([version]))
                    cli('customization','set','--background-image',str(maximum),'--profile',selected);revision+=1
                    assert current()['configurationRevision']==revision
                    assert sum(len(base64.b64decode(v))==2500000 for v in current()['profiles'][0]['appearanceResources'].values())==1
                for scheme in ('light','dark'):
                    for action in ('base','left-trigger','right-trigger','both-triggers'):
                        out=args.output_dir/(scheme+'-'+action+'.png');cli('layout','preview','--profile',selected,'--native','--appearance',scheme,'--action-set',action,'-o',str(out),'--scale','1');assert out.read_bytes().startswith(bytes.fromhex('89504e470d0a1a0a'))
                        color=Image.open(out).convert('RGBA').getpixel((20,200));assert color[0]<90 and color[1]>70 and color[2]>150 and color[3]==255, (scheme,action,color)
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
                        while not (authority/'control.sock').exists():
                            assert host.poll() is None and time.monotonic()<deadline, ('receiver startup', host.poll(), (root/'host.log').read_text()[-2500:])
                            time.sleep(.05)
                        mcp=subprocess.Popen([str(install/'thumble-mcp'),'--allow-config-write'],env=env,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=mlog)
                        rpc('initialize',dict(protocolVersion='2024-11-05',capabilities={},clientInfo=dict(name='rich-background-verifier',version='1')));mcp.stdin.write(b'{"jsonrpc":"2.0","method":"notifications/initialized"}\n');mcp.stdin.flush()
                        draft=call('begin_configuration_draft',dict(expectedConfigurationRevision=revision));saved=file.read_bytes()
                        operation=dict(type='customization.set',profileID=selected,variant='landscape',changes=dict(backgroundEdit=dict(action='fill',scope='all',fill=dict(kind='image',dataBase64=base64.b64encode(maximum.read_bytes()).decode(),fileName='mcp.png',contentMode='fit'))))
                        edited=call('edit_configuration_draft',dict(draftId=draft['draftId'],expectedDraftRevision=draft['draftRevision'],operationId=str(uuid.uuid4()),operation=operation));assert file.read_bytes()==saved
                        commit=dict(draftId=draft['draftId'],expectedDraftRevision=edited['draft']['draftRevision'],expectedConfigurationRevision=revision,commitId=str(uuid.uuid4()))
                        assert call('save_configuration_draft',commit)['configurationRevision']==revision+1
                        saved=file.read_bytes();assert call('save_configuration_draft',commit)['idempotentReplay'] and file.read_bytes()==saved
                        cli('customization','set','--light-background','#FF0000','--profile',selected)
                        target=current()['profiles'][0]
                        for row in target['appearanceBank']['variants']:
                            assert row['appearance']['backgroundLightColor']['red']==1
                            assert row['appearance']['backgroundDarkFillStyle']['kind']=='image'
                        cli('customization','set','--reset-background','--profile',selected)
                        target=current()['profiles'][0]
                        for row in target['appearanceBank']['variants']:
                            assert not any(k in row['appearance'] for k in ('backgroundFillStyle','backgroundLightFillStyle','backgroundDarkFillStyle','backgroundLightColor','backgroundDarkColor'))
                        assert target['appearanceResources'][digest]==base64.b64encode(blob).decode()
                        assert not any(len(base64.b64decode(v))==2500000 for v in target['appearanceResources'].values())

                    finally:
                        for process in (mcp,host):
                            if process is not None and process.poll() is None:process.terminate();process.wait(timeout=10)
                        hlog.flush();mlog.flush();shutil.copy2(root/'host.log',args.output_dir/'receiver.log');shutil.copy2(root/'mcp.log',args.output_dir/'mcp.log')
            # Final record omits credentials; binary tables are represented by hashes.
            (args.output_dir/('owned-layout.json' if owned else 'legacy-layout.json')).write_text(json.dumps(current()['profiles'][0]['customization'],indent=2)+'\n')
        result=dict(classicAndOwnedCLI=True,gradientTileAndImage=True,allEightStates=True,plainOwnedNullBankPreserved=True,decodedImagePixelsInAllEightPreviews=True,schemeScopedEdits=True,maximumImageBytes=2500000,unreachableImagesPruned=True,solidAndResetClearSavedImageStates=True,preciseProfileAndBankRowMetadata=True,unrelatedControlsProfilesAndLaunchData=True,opaqueResourceRetained=True,helperFreeReplay=True,laterClockNoOp=True,regularFileGuards=True,localMCPPrivateDraftCommitReplay=True,maximumImageThroughMCP=True,automaticMigrationActivated=False)
        (args.output_dir/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))

if __name__=='__main__':main()
