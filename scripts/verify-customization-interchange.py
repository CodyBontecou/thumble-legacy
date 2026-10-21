#!/usr/bin/env python3
"""Real private-home CLI/MCP customization interchange, without desktop storage."""
import argparse, base64, hashlib, json, os, select, shutil, subprocess, tempfile, time, uuid
from pathlib import Path

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ('cli','bridge','cli-bridge','host','mcp','output-dir'):parser.add_argument('--'+name,type=Path,required=True)
    args=parser.parse_args();args.output_dir.mkdir(parents=True,exist_ok=True)
    repo=Path(__file__).resolve().parent.parent
    fixture=json.loads((repo/'Host/fixtures/owned-configuration/v1/document.json').read_bytes())
    pixel=base64.b64decode('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Wl6X1cAAAAASUVORK5CYII=')
    with tempfile.TemporaryDirectory(prefix='thumble-customization-',dir='/private/tmp') as directory:
        root=Path(directory);install=root/'bin';install.mkdir(mode=0o700)
        for path,name in [(args.cli,'thumble'),(args.bridge,'native-helper'),(args.cli_bridge,'thumble-cli-bridge'),(args.host,'thumble-host'),(args.mcp,'thumble-mcp')]:shutil.copy2(path,install/name)
        wrapper=install/'thumble-bridge'
        wrapper.write_text('#!/usr/bin/python3\nimport json,subprocess,sys\nfrom pathlib import Path\nr=Path(__file__).parent\ndata=sys.stdin.buffer.read()\nv=json.loads(data)\np=subprocess.run([str(r/"native-helper")],input=data,capture_output=True)\nif v.get("operation",{}).get("type")=="customization.import" and "document" in v:\n (r/"legacy-request.json").write_bytes(data)\n (r/"legacy-response.json").write_bytes(p.stdout)\nsys.stdout.buffer.write(p.stdout)\nsys.stderr.buffer.write(p.stderr)\nsys.exit(p.returncode)\n');wrapper.chmod(0o700)
        home=root/'home';home.mkdir();authority=home/'Library/Application Support/ThumbleHost';authority.mkdir(parents=True)
        env=dict(os.environ,HOME=str(home),PATH=str(install)+os.pathsep+os.environ.get('PATH',''))
        for key in ('THUMBLE_HOST_STATE_DIR','THUMBLE_HOST_CONTROL_SOCKET','POCKETPAD_HOST_STATE_DIR','POCKETPAD_HOST_CONTROL_SOCKET'):env.pop(key,None)
        profiles=fixture['profiles'];source_id=profiles[0]['id'];target_id=profiles[1]['id']
        profiles[0]['customization']['futureLayoutPrecision']='LAYOUT_FRACTION';profiles[1]['futureDestinationPrecision']='DESTINATION_FRACTION'
        unused=base64.b64encode(b'private-unused-bookmark').decode();unused_hash=hashlib.sha256(b'private-unused-bookmark').hexdigest()
        profiles[0]['appearanceResources'][unused_hash]=unused
        for p in profiles:p['outputMode']='custom'
        launch=json.loads(json.dumps(profiles[0]['launchTarget']))
        for name in ('bookmarkData','iconPNGData'):
            launch[name]=profiles[0]['appearanceResources'][launch[name]['resourceSHA256']]
        launch['displayName']='Destination local app';profiles[1]['launchTarget']=launch
        state=dict(schemaVersion=4,serverID='private-customization-fixture',trustedClients={},configurationRevision=41,configurationUpdatedAt=100,recentConfigurationCommits=[])
        for name in ('profiles','activeProfileID','defaultProfileID','keyBindings','outputBindings','profileKeyBindings','profileOutputBindings'):
            if name in fixture:state[name]=fixture[name]
        state['profiles']=profiles
        file=authority/'state.json';file.write_text(json.dumps(state).replace('"LAYOUT_FRACTION"','0.12345678901234567890123456789').replace('"DESTINATION_FRACTION"','0.98765432109876543210987654321'));file.chmod(0o600)
        def current():return json.loads(file.read_bytes())
        def native(*arguments,allowed=True):
            result=subprocess.run([str(install/'thumble'),*arguments],env=env,capture_output=True,text=True,timeout=60)
            assert (result.returncode==0)==allowed,(arguments,result.stderr[-2200:])
            return result
        source=root/'banked.json';plain=root/'plain.json'
        saved=file.read_bytes();native('customization','export','--profile',source_id,'-o',str(source));native('customization','export','--profile',target_id,'-o',str(plain))
        assert file.read_bytes()==saved
        assert native('customization','show','--profile',source_id).stdout.rstrip()==source.read_text()
        assert '0.12345678901234567890123456789' in source.read_text()
        packet=json.loads(source.read_bytes());assert packet['schema']=='com.codybontecou.thumble.customization'
        assert len(packet['appearanceBank']['variants'])==8 and unused not in source.read_text() and unused_hash not in source.read_text()
        assert 'elements' in json.loads(plain.read_bytes())
        assert profiles[0]['launchTarget']['bookmarkData']['resourceSHA256'] not in source.read_text()
        before=current();invocation=str(uuid.uuid4());command=('customization','import',str(source),'--profile',target_id,'--invocation-id',invocation)
        native(*command);after=current();assert after['configurationRevision']==42 and after['profiles'][0]==before['profiles'][0]
        assert after['profiles'][1]['id']==target_id and after['profiles'][1]['name']==before['profiles'][1]['name']
        assert after['profiles'][1]['outputMode']==before['profiles'][1]['outputMode']
        local=after['profiles'][1]['launchTarget'];assert local['displayName']=='Destination local app'
        for name in ('bookmarkData','iconPNGData'):
            assert after['profiles'][1]['appearanceResources'][local[name]['resourceSHA256']]==launch[name]
        assert '0.98765432109876543210987654321' in file.read_text() and file.read_text().count('0.12345678901234567890123456789')==2
        assert after['activeProfileID']==before['activeProfileID'] and after['defaultProfileID']==before['defaultProfileID']
        exported=root/'transferred.json';native('customization','export','--profile',target_id,'-o',str(exported));assert exported.read_bytes()==source.read_bytes()
        saved=file.read_bytes();helper=install/'thumble-bridge';parked=install/'helper.parked';helper.rename(parked)
        try:native(*command);assert file.read_bytes()==saved
        finally:parked.rename(helper)
        body=current()['profiles'];native('customization','import',str(source),'--profile',target_id,'--invocation-id',str(uuid.uuid4()));assert current()['configurationRevision']==42 and current()['profiles']==body
        # Corrupt schema/digest and nonregular descriptors cannot modify authority.
        bad=root/'bad.json';bad.write_text(source.read_text().replace('"version":1','"version":2'));saved=file.read_bytes();native('customization','import',str(bad),'--profile',target_id,allowed=False);assert file.read_bytes()==saved
        link=root/'link.json';link.symlink_to(source);native('customization','import',str(link),allowed=False)
        fifo=root/'fifo';os.mkfifo(fifo);native('customization','import',str(fifo),allowed=False)
        # Classic files are prepared through the retained native codec/pooling path.
        layout=json.loads(plain.read_bytes());layout['backgroundFillStyle']=dict(kind='image',image=dict(data=base64.b64encode(pixel).decode(),contentMode='fill',opacity=1,exposure=0,contrast=0,saturation=0,temperature=0,tint=0,highlights=0,shadows=0))
        layout['futureClassicPrecision']='CLASSIC_FRACTION'
        plain.write_text(json.dumps(layout).replace(chr(34)+'CLASSIC_FRACTION'+chr(34),'0.11111111111111111111111111111'));native('customization','import',str(plain),'--profile',target_id,'--invocation-id',str(uuid.uuid4()))
        assert '0.11111111111111111111111111111' in file.read_text()
        assert current()['configurationRevision']==43 and len(current()['profiles'][1]['appearanceBank']['variants'])==8
        assert any(base64.b64decode(v)==pixel for v in current()['profiles'][1]['appearanceResources'].values())
        previews=[]
        for scheme in ('light','dark'):
            for action in ('base','left-trigger','right-trigger','both-triggers'):
                image=root/(scheme+'-'+action+'.png')
                native('layout','preview','--profile',target_id,'--native','--appearance',scheme,'--action-set',action,'-o',str(image),'--scale','1')
                assert image.read_bytes().startswith(bytes.fromhex('89504e470d0a1a0a'))
                previews.append(hashlib.sha256(image.read_bytes()).hexdigest())
                shutil.copy2(image,args.output_dir/image.name)
        host=None;mcp=None;sequence=0
        with (root/'host.log').open('w') as host_log,(root/'mcp.log').open('w') as mcp_log:
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
                host=subprocess.Popen([str(install/'thumble-host'),'run','--port','0','--no-bonjour','--no-input','--allow-config-write'],env=env,stdout=host_log,stderr=host_log)
                deadline=time.monotonic()+20
                while not (authority/'control.sock').exists():assert host.poll() is None and time.monotonic()<deadline;time.sleep(.05)
                mcp=subprocess.Popen([str(install/'thumble-mcp')],env=env,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=mcp_log)
                rpc('initialize',dict(protocolVersion='2024-11-05',capabilities={},clientInfo=dict(name='customization-verifier',version='1')));mcp.stdin.write(b'{"jsonrpc":"2.0","method":"notifications/initialized"}\n');mcp.stdin.flush()
                value=call('export_customization',dict(profileID=source_id));assert value['artifact']['artifactJSON']==source.read_text()
                params=dict(profileID=target_id,expectedConfigurationRevision=43,invocationID=str(uuid.uuid4()),artifactJSON=source.read_text())
                call('import_customization',params,allowed=False)
                mcp.terminate();mcp.wait(timeout=10)
                mcp=subprocess.Popen([str(install/'thumble-mcp'),'--allow-config-write'],env=env,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=mcp_log)
                rpc('initialize',dict(protocolVersion='2024-11-05',capabilities={},clientInfo=dict(name='customization-writer',version='1')));mcp.stdin.write(b'{"jsonrpc":"2.0","method":"notifications/initialized"}\n');mcp.stdin.flush()
                result=call('import_customization',params);assert result['outcome']['configurationRevision']==44
                saved=file.read_bytes();assert call('import_customization',params)['outcome']['idempotentReplay'] and file.read_bytes()==saved
                call('import_customization',dict(params,invocationID=str(uuid.uuid4())),allowed=False);assert file.read_bytes()==saved
            finally:
                for process in (mcp,host):
                    if process is not None and process.poll() is None:process.terminate();process.wait(timeout=10)
        # Valid inline appearance banks normalize known Data before joining a pool.
        inline=json.loads(source.read_bytes());pool=inline.pop('appearanceResources');inline.pop('appearanceResourceVersion')
        def expand(node):
            if isinstance(node,dict):
                if set(node)=={'resourceSHA256'}:return pool[node['resourceSHA256']]
                return {k:expand(v) for k,v in node.items()}
            if isinstance(node,list):return [expand(v) for v in node]
            return node
        inline=expand(inline);inline['customization']['futureLayoutPrecision']='INLINE_FRACTION'
        inline_file=root/'inline-bank.json';inline_file.write_text(json.dumps(inline).replace('"INLINE_FRACTION"','0.12345678901234567890123456789'))
        native('customization','import',str(inline_file),'--profile',target_id,'--invocation-id',str(uuid.uuid4()))
        assert current()['configurationRevision']==45 and '0.12345678901234567890123456789' in file.read_text()
        for scheme in ('light','dark'):
            for action in ('base','left-trigger','right-trigger','both-triggers'):
                image=args.output_dir/('inline-'+scheme+'-'+action+'.png')
                native('layout','preview','--profile',target_id,'--native','--appearance',scheme,'--action-set',action,'-o',str(image),'--scale','1')
                assert image.read_bytes().startswith(bytes.fromhex('89504e470d0a1a0a'))
        # The same ordinary bare layout remains usable on unmigrated authority.
        legacy_home=root/'legacy';legacy_dir=legacy_home/'Library/Application Support/ThumbleHost';legacy_dir.mkdir(parents=True);env['HOME']=str(legacy_home)
        profile=json.loads(json.dumps(before['profiles'][1]));profile.pop('profileFormatVersion',None);profile['orientationPreference']='landscape';profile.pop('futureDestinationPrecision',None)
        legacy=dict(schemaVersion=3,serverID='private-classic-customization',trustedClients={},configurationRevision=41,configurationUpdatedAt=100,recentConfigurationCommits=[],profiles=[profile],activeProfileID=target_id,defaultProfileID=target_id,keyBindings={},outputBindings={},profileKeyBindings={},profileOutputBindings={})
        file=legacy_dir/'state.json';file.write_text(json.dumps(legacy));file.chmod(0o600)
        legacy_layout=json.loads(plain.read_bytes());legacy_layout.pop('futureClassicPrecision',None);legacy_file=root/'legacy-layout.json';legacy_file.write_text(json.dumps(legacy_layout).replace('0.6','0.59999999999999998',1))
        native('customization','import',str(legacy_file),'--profile',target_id,'--invocation-id',str(uuid.uuid4()));assert current()['configurationRevision']==42
        native('customization','export','--profile',target_id,'-o',str(exported));assert 'elements' in json.loads(exported.read_bytes())
        for name in ('legacy-request.json','legacy-response.json'):shutil.copy2(install/name,args.output_dir/name)
        saved=file.read_bytes();native('customization','import',str(plain),'--profile',target_id,allowed=False);assert file.read_bytes()==saved
        saved=file.read_bytes();native('customization','import',str(source),'--profile',target_id,allowed=False);assert file.read_bytes()==saved
        result=dict(exactCLIInspectionExportImport=True,localMCPInterchange=True,eightStateAndResourceTransfer=True,inlineBankNativeRoundtrip=True,preciseMetadata=True,identitySelectionAndUnrelatedProfiles=True,privateUnusedResourcesExcluded=True, destinationLaunchAssociationPreserved=True,classicNativePreparation=True, eightNativePreviewSHA256=previews, readOnlyMCPExportWithoutOptIn=True,helperFreeReplay=True,identicalImportNoOp=True,boundedFileGuards=True,legacyClassicInterchange=True,pooledToLegacyRefused=True,automaticMigrationActivated=False)
        (args.output_dir/'result.json').write_text(json.dumps(result,indent=2)+'\n');shutil.copy2(source,args.output_dir/'banked.json');shutil.copy2(plain,args.output_dir/'classic.json');print(json.dumps(result))

if __name__=='__main__':main()
