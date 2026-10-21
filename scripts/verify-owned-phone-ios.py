#!/usr/bin/env python3
"""Run the actual iOS client against an isolated, no-input native receiver.

Uses an already booted QA simulator and a built hosted XCTest bundle. The test
restores simulator preferences and keeps its owned cache in a disposable folder.
"""
import argparse
import copy
import json
import os
import plistlib
from pathlib import Path
import shutil
import subprocess
import tempfile
import time


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--host',type=Path,required=True)
    parser.add_argument('--bridge',type=Path,required=True)
    parser.add_argument('--xctestrun',type=Path,required=True)
    parser.add_argument('--simulator',required=True)
    args=parser.parse_args()
    repo=Path(__file__).resolve().parent.parent
    with tempfile.TemporaryDirectory(prefix='thumble-ios-wire-',dir='/private/tmp') as directory:
        root=Path(directory);install=root/'bin';install.mkdir(mode=0o700)
        for source,name in ((args.host,'thumble-host'),(args.bridge,'thumble-bridge')):
            shutil.copy2(source.resolve(strict=True),install/name)
        authority=root/'authority';authority.mkdir(mode=0o700)
        document=json.loads((repo/'Host/fixtures/owned-element-sequences/v1/after-reset-and-hide.json').read_bytes())
        second=copy.deepcopy(document['profiles'][0]);second['id']='00000000-0000-0000-0000-00000000b002';second['name']='Character B fixture'
        document['profiles'].append(second)
        token='isolated-ios-test-token'
        server='isolated-ios-test-mac'
        state=dict(schemaVersion=4,serverID=server,trustedClients={token:dict(name='iOS test',createdAt=100,lastSeenAt=100)},configurationRevision=41,
            configurationUpdatedAt=100,recentConfigurationCommits=[],futureState=9007199254740993,
            ownedConfigurationExtensions=dict(iosFixture=True))
        for field in ('profiles','activeProfileID','defaultProfileID','keyBindings','outputBindings','profileKeyBindings','profileOutputBindings'):
            if field in document: state[field]=document[field]
        for profile in state['profiles']: profile['phonePrecise']='PHONE_PRECISE'
        state_file=authority/'state.json';state_file.write_text(json.dumps(state).replace('"PHONE_PRECISE"','1.0000000000000000000001'));state_file.chmod(0o600)
        env=dict(os.environ,THUMBLE_HOST_STATE_DIR=str(authority),THUMBLE_HOST_CONTROL_SOCKET=str(root/'control.sock'))
        process=None
        try:
            with (root/'receiver.log').open('w') as log:
                process=subprocess.Popen([str(install/'thumble-host'),'run','--port','0','--no-bonjour','--no-input'],env=env,stdout=log,stderr=log)
                deadline=time.monotonic()+30
                while True:
                    assert process.poll() is None,(root/'receiver.log').read_text()[-2000:]
                    pid=authority/'runtime.pid';metadata=authority/'runtime.json'
                    if pid.is_file() and pid.read_text().strip()==str(process.pid) and metadata.is_file():
                        runtime=json.loads(metadata.read_bytes())
                        if runtime.get('actualPort'):
                            assert runtime['inputEnabled'] is False and runtime['configurationWriteEnabled'] is False
                            break
                    assert time.monotonic()<deadline,'Isolated receiver did not become ready'
                    time.sleep(.05)
                original_run=args.xctestrun.resolve(strict=True)
                configuration=plistlib.loads(original_run.read_bytes())
                def absolute_roots(value):
                    if isinstance(value,str): return value.replace('__TESTROOT__',str(original_run.parent))
                    if isinstance(value,list): return [absolute_roots(item) for item in value]
                    if isinstance(value,dict): return {key:absolute_roots(item) for key,item in value.items()}
                    return value
                configuration=absolute_roots(configuration)
                if 'TestConfigurations' in configuration:
                    targets=[target for item in configuration['TestConfigurations'] for target in item.get('TestTargets',[])]
                else:
                    targets=[value for key,value in configuration.items() if not key.startswith('__')]
                selected=[target for target in targets if target.get('BlueprintName')=='ThumbleiOSProtocolTests']
                assert len(selected)==1,'Expected the dedicated iOS protocol test bundle'
                selected[0].setdefault('EnvironmentVariables',{}).update(dict(
                    THUMBLE_DISABLE_STARTUP_SMART_CONNECT='1',
                    THUMBLE_OWNED_RECEIVER_PORT=str(runtime['actualPort']),
                    THUMBLE_OWNED_RECEIVER_SERVER=server,
                    THUMBLE_OWNED_RECEIVER_TOKEN=token,
                    THUMBLE_OWNED_RECEIVER_SECOND_PROFILE=second['id']))
                test_run=root/'isolated.xctestrun';test_run.write_bytes(plistlib.dumps(configuration))
                result=subprocess.run(['xcodebuild','test-without-building','-xctestrun',str(test_run),
                    '-destination',f'platform=iOS Simulator,id={args.simulator}',
                    '-only-testing:ThumbleiOSProtocolTests/OwnedPhoneClientTests/testActualHeadlessReceiverCommitsEditsAndProfileSelection',
                    'CODE_SIGNING_ALLOWED=NO'],capture_output=True,text=True,timeout=180)
                assert result.returncode==0 and '** TEST EXECUTE SUCCEEDED **' in result.stdout, result.stdout[-7000:]+result.stderr[-1000:]+'\n'+(root/'receiver.log').read_text()[-2000:]
                saved=json.loads(state_file.read_bytes())
                # Geometry, appearance, selection B and selection A each commit once.
                assert saved['configurationRevision']==45,saved['configurationRevision']
                assert saved['futureState']==9007199254740993 and saved['ownedConfigurationExtensions']==state['ownedConfigurationExtensions']
                assert saved['activeProfileID']==state['activeProfileID'] and saved['defaultProfileID']==state['defaultProfileID']
                assert len(saved['profiles'])==len(state['profiles'])
                assert state_file.read_text().count('1.0000000000000000000001')==len(state['profiles'])
                print(json.dumps(dict(iosClient=True,realHeadlessReceiver=True,nativeHelper=True,inputDisabled=True,
                    geometryCommitted=True,appearanceCommitted=True,explicitReceipts=True,selectionABA=True,
                    unrelatedMetadataPreserved=True,configurationRevision=45,productionAdmissionPolicy=True)))
        finally:
            if process is not None and process.poll() is None:
                process.terminate()
                try: process.wait(timeout=10)
                except subprocess.TimeoutExpired: process.kill();process.wait(timeout=5)


if __name__=='__main__': main()
