#!/usr/bin/env python3
"""Prove real Pixel journal drain -> exact migration -> owned reconnect.

All receiver state and phone storage are disposable test fixtures; input is off.
The installed app/test APKs must already contain the matching production client.
"""
import argparse
import base64
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
    for name in ('host','bridge','adb'): parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--serial',required=True)
    args=parser.parse_args()
    repo=Path(__file__).resolve().parent.parent
    with tempfile.TemporaryDirectory(prefix='thumble-phone-migration-',dir='/private/tmp') as directory:
        root=Path(directory);binary=root/'bin';binary.mkdir(mode=0o700)
        for source,name in ((args.host,'thumble-host'),(args.bridge,'thumble-bridge')): shutil.copy2(source.resolve(strict=True),binary/name)
        authority=root/'state';authority.mkdir(mode=0o700)
        profile=json.loads((repo/'Host/fixtures/legacy-configuration-migration/v1/plain-profile.json').read_bytes())
        token='disposable-migration-token';server='disposable-migration-mac'
        state=dict(schemaVersion=3,serverID=server,trustedClients={token:dict(name='Migration fixture phone',createdAt=100,lastSeenAt=100)},configurationRevision=41,configurationUpdatedAt=100,recentConfigurationCommits=[],profiles=[profile],activeProfileID=profile['id'],defaultProfileID=profile['id'],keyBindings={},outputBindings={},profileKeyBindings={},profileOutputBindings={},unrelatedState=9007199254740993)
        state_file=authority/'state.json';state_file.write_text(json.dumps(state));state_file.chmod(0o600)
        env=dict(os.environ,THUMBLE_HOST_STATE_DIR=str(authority),THUMBLE_HOST_CONTROL_SOCKET=str(root/'control.sock'))
        receiver=None;instrument=None;forward=None;forwarded=False
        def host(*arguments):
            result=subprocess.run([str(binary/'thumble-host'),*arguments],env=env,capture_output=True,text=True,timeout=40)
            assert result.returncode==0,result.stderr[-2000:]
            return json.loads(result.stdout) if result.stdout.strip().startswith('{') else None
        def adb(*arguments):
            result=subprocess.run([str(args.adb),'-s',args.serial,*arguments],capture_output=True,text=True,timeout=30)
            assert result.returncode==0,result.stdout[-1500:]+result.stderr[-1000:]
            return result.stdout
        def start(port):
            nonlocal receiver
            log=open(root/'receiver.log','a')
            receiver=subprocess.Popen([str(binary/'thumble-host'),'run','--port',str(port),'--no-bonjour','--no-input'],env=env,stdout=log,stderr=log);log.close()
            until=time.monotonic()+25
            while True:
                assert receiver.poll() is None,(root/'receiver.log').read_text()[-2500:]
                pid=authority/'runtime.pid';metadata=authority/'runtime.json'
                if pid.is_file() and pid.read_text().strip()==str(receiver.pid) and metadata.is_file():
                    runtime=json.loads(metadata.read_bytes())
                    if runtime.get('actualPort'): return runtime['actualPort']
                assert time.monotonic()<until,'Receiver startup timed out'
                time.sleep(.05)
        try:
            port=start(0)
            with socket.socket() as probe: probe.bind(('127.0.0.1',0));forward=probe.getsockname()[1]
            adb('reverse','--no-rebind',f'tcp:{forward}',f'tcp:{port}');forwarded=True
            command=[str(args.adb),'-s',args.serial,'shell','am','instrument','-w','-e','class','com.thumble.android.OwnedPhoneEditTransportDeviceTest#physicalClientDrainsLegacyJournalThenRejoinsMigratedReceiver','-e','migrationReceiverURL',f'ws://127.0.0.1:{forward}','-e','migrationReceiverServerID',server,'-e','migrationReceiverToken',token,'-e','migrationProfileBase64',base64.b64encode(json.dumps(profile).encode()).decode(),'com.thumble.android.test/androidx.test.runner.AndroidJUnitRunner']
            # Spool output to a private file so verbose test errors cannot block
            # the device while the driver observes the independent receiver.
            with open(root/'instrumentation.log','w') as log:
                instrument=subprocess.Popen(command,stdout=log,stderr=subprocess.STDOUT)
            until=time.monotonic()+35;initial_offer_blocked=False
            while True:
                if instrument.poll() is not None:
                    print((root/'instrumentation.log').read_text(), flush=True)
                    print('AUTHORITY_DIAGNOSTIC='+json.dumps(json.loads(state_file.read_bytes())['profiles'][0]['customization']),flush=True)
                    raise AssertionError('Phone test exited before migration: '+(root/'receiver.log').read_text()[-2000:])
                status=host('status','--json')
                if status['core']['paired'] and not status['core']['clientSupportsOwnedProfiles']: initial_offer_blocked=True
                saved=json.loads(state_file.read_bytes())
                if saved['configurationRevision']==42 and status['core']['paired'] and status['core']['clientSupportsOwnedProfiles']: break
                assert time.monotonic()<until,'Phone did not drain and readmit legacy journal'
                time.sleep(.05)
            assert initial_offer_blocked,'Never observed the pending journal blocking owned admission'
            assert not status['inputEnabled'] and not status['configurationWriteEnabled']
            assert saved['profiles'][0]['customization']['buttonCustomizations']
            legacy_committed=state_file.read_bytes()
            host('stop');receiver.wait(timeout=15)
            plan=host('configuration','migration','plan')
            review=root/'plan.json';review.write_text(json.dumps(plan));review.chmod(0o600)
            receipt=host('configuration','migration','apply','--plan-file',str(review),'--allow-config-write')
            assert receipt['phase']=='committed' and receipt['baseRevision']==42 and receipt['resultRevision']==43
            backup=authority/'owned-migrations-v1'/receipt['sourceSha256']/'source-state.json'
            assert backup.read_bytes()==legacy_committed
            assert start(port)==port
            instrument.wait(timeout=75)
            output=(root/'instrumentation.log').read_text()
            assert instrument.returncode==0 and 'OK (1 test)' in output,output[-7000:]+'\n'+(root/'receiver.log').read_text()[-2500:]
            saved=json.loads(state_file.read_bytes())
            assert saved['schemaVersion']==4 and saved['configurationRevision']==44
            assert saved['serverID']==server and saved['trustedClients'].keys()==state['trustedClients'].keys()
            assert saved['profiles'][0]['id']==profile['id'] and saved['activeProfileID']==profile['id']==saved['defaultProfileID']
            assert saved['unrelatedState']==9007199254740993
            assert saved['profiles'][0]['customization']['colorSchemePreference']=='dark'
            print(json.dumps(dict(realPixelClient=True,productionAdmission=True,legacyJournalDrainedBeforeMigration=True,initialOwnedOfferBlocked=True,sameTrustReconnected=True,exactBackupAfterLegacyCommit=True,ownedEditCommitted=True,configurationRevision=44,inputDisabled=True,installedAuthorityTouched=False)))
        finally:
            if instrument is not None and instrument.poll() is None:
                instrument.terminate()
                try: instrument.wait(timeout=5)
                except subprocess.TimeoutExpired: instrument.kill();instrument.wait(timeout=5)
            if forwarded: subprocess.run([str(args.adb),'-s',args.serial,'reverse','--remove',f'tcp:{forward}'],capture_output=True,timeout=10)
            if receiver is not None and receiver.poll() is None:
                receiver.terminate()
                try: receiver.wait(timeout=10)
                except subprocess.TimeoutExpired: receiver.kill();receiver.wait(timeout=5)


if __name__=='__main__': main()
