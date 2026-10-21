#!/usr/bin/env python3
"""Run the actual Android client against an isolated, no-input native receiver.

Uses an already connected device and installed debug app/test APKs. Real saved
phone trust is untouched: the instrumentation constructs its own memory store.
"""
import argparse
import copy
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import tempfile
import time


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--host',type=Path,required=True)
    parser.add_argument('--bridge',type=Path,required=True)
    parser.add_argument('--adb',type=Path,required=True)
    parser.add_argument('--serial',required=True)
    args=parser.parse_args()
    repo=Path(__file__).resolve().parent.parent
    with tempfile.TemporaryDirectory(prefix='thumble-android-wire-',dir='/private/tmp') as directory:
        root=Path(directory);install=root/'bin';install.mkdir(mode=0o700)
        for source,name in ((args.host,'thumble-host'),(args.bridge,'thumble-bridge')):
            shutil.copy2(source.resolve(strict=True),install/name)
        authority=root/'authority';authority.mkdir(mode=0o700)
        document=json.loads((repo/'Host/fixtures/owned-element-sequences/v1/after-reset-and-hide.json').read_bytes())
        second=copy.deepcopy(document['profiles'][0]);second['id']='00000000-0000-0000-0000-00000000b002';second['name']='Character B fixture'
        document['profiles'].append(second)
        token='isolated-android-test-token'
        server='isolated-android-test-mac'
        state=dict(schemaVersion=4,serverID=server,trustedClients={token:dict(name='Android test',createdAt=100,lastSeenAt=100)},configurationRevision=41,
            configurationUpdatedAt=100,recentConfigurationCommits=[],futureState=9007199254740993,
            ownedConfigurationExtensions=dict(androidFixture=True))
        for field in ('profiles','activeProfileID','defaultProfileID','keyBindings','outputBindings','profileKeyBindings','profileOutputBindings'):
            if field in document: state[field]=document[field]
        for profile in state['profiles']: profile['phonePrecise']='PHONE_PRECISE'
        state_file=authority/'state.json';state_file.write_text(json.dumps(state).replace('"PHONE_PRECISE"','1.0000000000000000000001'));state_file.chmod(0o600)
        env=dict(os.environ,THUMBLE_HOST_STATE_DIR=str(authority),THUMBLE_HOST_CONTROL_SOCKET=str(root/'control.sock'))
        process=None;forward=None;forwarded=False
        def adb(*arguments,timeout=30):
            result=subprocess.run([str(args.adb),'-s',args.serial,*arguments],capture_output=True,text=True,timeout=timeout)
            assert result.returncode==0,result.stdout[-3000:]+result.stderr[-1000:]
            return result.stdout
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
                # Only this unique reverse mapping is removed during cleanup.
                with socket.socket() as probe:
                    probe.bind(('127.0.0.1',0));forward=probe.getsockname()[1]
                adb('reverse','--no-rebind',f'tcp:{forward}',f'tcp:{runtime["actualPort"]}')
                forwarded=True
                output=adb('shell','am','instrument','-w','-e','class','com.thumble.android.OwnedPhoneEditTransportDeviceTest#physicalClientUsesRealReceiverAndNativeHelper',
                    '-e','ownedReceiverURL',f'ws://127.0.0.1:{forward}','-e','ownedReceiverServerID',server,
                     '-e','ownedReceiverToken',token,'-e','ownedReceiverSecondProfileID',second['id'],
                    '-e','ownedReceiverProfileCount',str(len(state['profiles'])),
                    'com.thumble.android.test/androidx.test.runner.AndroidJUnitRunner',timeout=100)
                assert 'OK (1 test)' in output,output[-8000:]+'\n'+(root/'receiver.log').read_text()[-3000:]
                saved=json.loads(state_file.read_bytes())
                # Geometry, appearance, selection B and selection A each commit once.
                assert saved['configurationRevision']==45,saved['configurationRevision']
                assert saved['futureState']==9007199254740993 and saved['ownedConfigurationExtensions']==state['ownedConfigurationExtensions']
                assert saved['activeProfileID']==state['activeProfileID'] and saved['defaultProfileID']==state['defaultProfileID']
                assert len(saved['profiles'])==len(state['profiles'])
                assert state_file.read_text().count('1.0000000000000000000001')==len(state['profiles'])
                print(json.dumps(dict(androidClient=True,realHeadlessReceiver=True,nativeHelper=True,inputDisabled=True,
                    geometryCommitted=True,appearanceCommitted=True,explicitReceipts=True,selectionABA=True,
                    unrelatedMetadataPreserved=True,configurationRevision=45,productionAdmissionPolicy=True)))
        finally:
            if forwarded and forward is not None:
                subprocess.run([str(args.adb),'-s',args.serial,'reverse','--remove',f'tcp:{forward}'],capture_output=True,timeout=10)
            if process is not None and process.poll() is None:
                process.terminate()
                try: process.wait(timeout=10)
                except subprocess.TimeoutExpired: process.kill();process.wait(timeout=5)


if __name__=='__main__': main()
