package com.thumble.android

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import java.math.BigInteger
import java.net.InetAddress
import java.util.UUID
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.atomic.AtomicReference

@RunWith(AndroidJUnit4::class)
class OwnedPhoneEditTransportDeviceTest {
    private fun profile()=ControllerProfile.parse(JSONObject(InstrumentationRegistry.getInstrumentation().context.assets
        .open("owned-configuration/v1/document.json").bufferedReader().use { it.readText() }).getJSONArray("profiles").getJSONObject(0))
    private fun await(label: String,timeoutMs: Long=8000,predicate: ()->Boolean) {
        val until=android.os.SystemClock.elapsedRealtime()+timeoutMs
        while(!predicate() && android.os.SystemClock.elapsedRealtime()<until) android.os.SystemClock.sleep(10)
        assertTrue(label,predicate())
    }
    private fun snapshot(profile: ControllerProfile,revision: Long,type: String="gamepad_profiles")=WireCodec.message(type)
        .put("serverID","mac-a").put("profileFormatVersion",5).put("configurationRevision",revision)
        .put("inputProtocolVersion",3).put("authToken","fixture-only-not-a-real-pairing")
        .put("capabilities",JSONArray().put("owned_phone_edit_v1"))
        .put("gamepadProfiles",JSONArray().put(profile.source)).put("gamepadProfileID",profile.id.toString())
        .put("defaultGamepadProfileID",profile.id.toString())
    private class Peer(onMessage: (WebSocket,JSONObject)->Unit): AutoCloseable {
        val server=MockWebServer();val received=CopyOnWriteArrayList<JSONObject>();val socket=AtomicReference<WebSocket>()
        init {
            server.enqueue(MockResponse().withWebSocketUpgrade(object:WebSocketListener() {
                override fun onOpen(ws:WebSocket,response:okhttp3.Response) { socket.set(ws) }
                override fun onMessage(ws:WebSocket,text:String) { val message=WireCodec.decode(text.toByteArray(),exactNumbers=true);received.add(message);onMessage(ws,message) }
            }))
            server.start(InetAddress.getByName("127.0.0.1"),0)
        }
        fun target()=PairingTarget(listOf("ws://127.0.0.1:${server.port}"),"123456","mac-a")
        override fun close() { socket.get()?.close(1000,"Finished");server.shutdown() }
    }
    private fun receipt(intent: JSONObject,status: String,current: Long,committed: Long?=null)=JSONObject()
        .put("schemaVersion",1).put("editID",intent.getString("editID")).put("profileID",intent.getString("profileID"))
        .put("baseConfigurationRevision",intent.get("baseConfigurationRevision")).put("configurationRevision",current)
        .put("status",status).put("changed",committed!=null && BigInteger.valueOf(committed)>OwnedPhoneEdit.revision(intent.get("baseConfigurationRevision")))
        .also { if(committed!=null) it.put("committedRevision",committed) else it.put("errorCode","owned_phone_revision_conflict") }
    private fun result(receipt: JSONObject)=WireCodec.message("owned_phone_edit_result").put("profileFormatVersion",5).put("ownedPhoneEditResult",receipt)

    @Test fun snapshotEchoDoesNotAcknowledgeAnEditAndCharacterSwapsKeepItsJournal() {
        val profile=profile();val store=MemorySessionStore();val state=AtomicReference(ClientState())
        val persistedBeforeSend=AtomicReference(false)
        Peer { ws,message ->
            when(message.getString("type")) {
                "hello","pairing_request" -> ws.send(snapshot(profile,41,"pairing_accepted").toString())
                "owned_phone_edit" -> persistedBeforeSend.set(store.loadPendingOwnedPhoneEdits().size==1)
            }
        }.use { peer ->
            val client=ControllerClient(store,enableOwnedPhoneEdits=true) { state.set(it) }
            try {
                client.connect(peer.target());await("paired format 5") { state.get().paired }
                client.beginLayoutEditing(profile.id,LayoutOrientation.LANDSCAPE)
                await("editor open") { state.get().layoutEditing }
                val editing=state.get()
                val moved=LayoutControlPatch(profile.controls(true).first().id,mapOf("centerX" to .63)).applying(editing.layoutEditEntry!!)
                client.commitLayoutEdit(moved,editing.layoutEditToken)
                await("owned intent sent after persistence") { peer.received.any { it.optString("type")=="owned_phone_edit" } && persistedBeforeSend.get() }
                val packet=peer.received.single { it.optString("type")=="owned_phone_edit" }
                assertFalse(packet.has("gamepadCustomization"));assertFalse(packet.has("gamepadProfiles"))
                val pending=store.loadPendingOwnedPhoneEdits().single()
                assertEquals(pending.edit.json().toString(),packet.getJSONObject("ownedPhoneEdit").toString())
                val optimistic=store.loadSnapshot().selected!!
                peer.socket.get().send(snapshot(optimistic,42).toString())
                await("snapshot received") { state.get().snapshot.configurationRevision==BigInteger.valueOf(42) }
                assertEquals("snapshot is not an explicit receipt",1,store.loadPendingOwnedPhoneEdits().size)
                val other=ControllerProfile.parse(JSONObject(profile.source.toString()).put("id",UUID.randomUUID().toString()).put("name","Other Character"))
                val swapped=snapshot(other,43).put("gamepadProfiles",JSONArray().put(optimistic.source).put(other.source))
                    .put("defaultGamepadProfileID",profile.id.toString())
                peer.socket.get().send(swapped.toString())
                await("character B selected") { state.get().snapshot.selectedID==other.id }
                assertFalse(state.get().layoutEditing);assertEquals(profile.id,state.get().snapshot.defaultID)
                assertEquals(1,store.loadPendingOwnedPhoneEdits().size)
                peer.socket.get().send(snapshot(optimistic,44).put("gamepadProfiles",JSONArray().put(optimistic.source).put(other.source)).toString())
                await("character A restored") { state.get().snapshot.selectedID==profile.id }
                peer.socket.get().send(result(receipt(packet.getJSONObject("ownedPhoneEdit"),"applied",44,42)).toString())
                await("matching receipt retired edit") { store.loadPendingOwnedPhoneEdits().isEmpty() && state.get().pendingLayoutChanges==0 }
                client.setAppearancePreference(AppearancePreference.DARK)
                await("appearance uses scalar intent") { peer.received.count { it.optString("type")=="owned_phone_edit" }==2 }
                val appearance=peer.received.last { it.optString("type")=="owned_phone_edit" }.getJSONObject("ownedPhoneEdit")
                assertEquals("dark",appearance.getString("colorScheme"))
                assertFalse(appearance.has("appearanceBank"));assertFalse(appearance.has("controlEdits"))
                peer.socket.get().send(result(receipt(appearance,"rejected",44)).toString())
                await("rejection kept for review") { state.get().error?.contains("recovery data")==true }
                assertEquals(1,store.loadPendingOwnedPhoneEdits().size)
            } finally { client.dispose() }
        }
    }
    @Test fun reconnectReplaysExactIntentThenExplicitConflictRetriesAgainstFreshMetadata() {
        val original=profile();val before=LayoutSnapshot.resolved(original,LayoutOrientation.LANDSCAPE)
        val moved=LayoutControlPatch(original.controls(true).first().id,mapOf("centerX" to .67)).applying(before)
        val intent=OwnedPhoneEdit.between(original.id,before,moved,BigInteger.valueOf(41))!!
        val local=intent.applying(original,100)
        val pending=PendingOwnedPhoneEdit.capture("mac-a",intent,100,local.source.toString())
        val store=MemorySessionStore()
        store.saveOwnedEditingState(ProfileSnapshot(listOf(local),local.id,local.id,serverID="mac-a",configurationRevision=BigInteger.valueOf(41)),emptyList(),emptyList(),emptyList(),listOf(pending))
        val fresh=ControllerProfile.parse(JSONObject(original.source.toString()).put("name","New name from Mac").put("futureSyncMetadata","keep"))
        val state=AtomicReference(ClientState())
        Peer { ws,message ->
            when(message.getString("type")) {
                "hello","pairing_request" -> ws.send(snapshot(fresh,42,"pairing_accepted").toString())
                "owned_phone_edit" -> {
                    val request=message.getJSONObject("ownedPhoneEdit")
                    if(request.getString("editID")==intent.editID.toString())
                        ws.send(result(receipt(request,"conflict",42)).toString())
                }
                "gamepad_profiles" -> ws.send(snapshot(fresh,42).toString())
            }
        }.use { peer ->
            val client=ControllerClient(store,enableOwnedPhoneEdits=true) { state.set(it) }
            try {
                client.connect(peer.target())
                await("replayed and rebased requests") { peer.received.count { it.optString("type")=="owned_phone_edit" }==2 }
                val requests=peer.received.filter { it.optString("type")=="owned_phone_edit" }.map { it.getJSONObject("ownedPhoneEdit") }
                assertEquals(intent.json().toString(),requests[0].toString())
                assertNotEquals(requests[0].getString("editID"),requests[1].getString("editID"))
                assertEquals(BigInteger.valueOf(42),OwnedPhoneEdit.revision(requests[1].get("baseConfigurationRevision")))
                val retained=JSONObject(store.loadPendingOwnedPhoneEdits().single().recoveryProfileJSON())
                assertEquals("New name from Mac",retained.getString("name"));assertEquals("keep",retained.getString("futureSyncMetadata"))
                peer.socket.get().send(result(receipt(requests[1],"applied",43,43)).toString())
                await("retry acknowledged") { store.loadPendingOwnedPhoneEdits().isEmpty() }
            } finally { client.dispose() }
        }
    }
    @Test fun oldPendingEditsPreventFormatFiveAdmissionWithoutBeingErased() {
        val profile=profile();val store=MemorySessionStore()
        val pending=PendingSkinSelection(profile.id,null,100,"mac-a")
        store.saveSkinState(ProfileSnapshot(listOf(profile),profile.id,profile.id,serverID="mac-a"),listOf(pending))
        val state=AtomicReference(ClientState())
        Peer { ws,message -> if(message.optString("type") in setOf("hello","pairing_request"))
            ws.send(snapshot(profile,42,"pairing_accepted").toString()) }.use { peer ->
            val client=ControllerClient(store,enableOwnedPhoneEdits=true) { state.set(it) }
            try {
                client.connect(peer.target())
                await("format gate rejected owned connection") { peer.received.isNotEmpty() && state.get().error?.contains("unsupported profile")==true }
                assertEquals("[4]",peer.received.first().getJSONArray("supportedProfileFormats").toString())
                assertEquals(listOf(pending),store.loadPendingSkinSelections())
                assertEquals(profile.id,store.loadSnapshot().selectedID)
                assertTrue(peer.received.none { it.optString("type")=="owned_phone_edit" })
                assertFalse(state.get().paired)
            } finally { client.dispose() }
        }
    }

    @Test fun legacyEditRecordedAfterHelloPreventsOwnedSnapshotReplacement() {
        val owned=profile()
        val legacy=ControllerProfile.parse(JSONObject().put("id",owned.id.toString()).put("name","Offline legacy")
            .put("customization",owned.source.getJSONObject("customization")))
        val store=MemorySessionStore();store.saveSnapshot(ProfileSnapshot(listOf(legacy),legacy.id,legacy.id,serverID="mac-a"))
        val state=AtomicReference(ClientState());val completed=AtomicReference<Boolean>()
        Peer { _,_ -> }.use { peer ->
            val client=ControllerClient(store) { state.set(it) }
            try {
                client.connect(peer.target())
                await("clean hello sent") { peer.received.isNotEmpty() }
                assertEquals("[4,5]",peer.received.first().getJSONArray("supportedProfileFormats").toString())
                val removal=SkinReference("com.thumble.race","1.0.0")
                client.queueSkinRemoval(removal) { completed.set(it) }
                await("offline removal persisted during handshake") { completed.get()==true && store.loadPendingSkinRemovals().size==1 }
                val pending=store.loadPendingSkinRemovals()
                val cached=store.loadSnapshot().selected!!.source.toString()
                peer.socket.get().send(snapshot(owned,41,"pairing_accepted").toString())
                await("late pending edit rejected owned snapshot") { state.get().error?.contains("need reconciliation")==true }
                assertFalse(state.get().paired)
                assertEquals(pending,store.loadPendingSkinRemovals())
                assertEquals(cached,store.loadSnapshot().selected!!.source.toString())
                assertFalse(store.loadSnapshot().selected!!.isOwned)
            } finally { client.dispose() }
        }
    }
    @Test fun removingUnusedLocalPackageFromOwnedCacheDoesNotQueueLegacyWork() {
        val owned=profile();val store=MemorySessionStore()
        store.saveSnapshot(ProfileSnapshot(listOf(owned),owned.id,owned.id,serverID="mac-a",configurationRevision=BigInteger.valueOf(41)))
        store.saveMac(TrustedMac("mac-a","fixture-token",listOf("ws://127.0.0.1:1")))
        val state=AtomicReference(ClientState());val completed=AtomicReference<Boolean>()
        val client=ControllerClient(store) { state.set(it) }
        try {
            await("owned cache restored") { state.get().snapshot.selected?.isOwned==true }
            client.queueSkinRemoval(SkinReference("com.thumble.unused","1.0.0")) { completed.set(it) }
            await("unused package removal allowed") { completed.get()==true }
            assertTrue(store.loadPendingSkinRemovals().isEmpty())
            assertTrue(store.loadPendingOwnedPhoneEdits().isEmpty())
            assertEquals(owned.source.toString(),store.loadSnapshot().selected!!.source.toString())
        } finally { client.dispose() }
    }

    @Test fun physicalClientDrainsLegacyJournalThenRejoinsMigratedReceiver() {
        val args=InstrumentationRegistry.getArguments()
        val url=args.getString("migrationReceiverURL")
        org.junit.Assume.assumeTrue("Run through verify-phone-authority-migration.py",url!=null)
        val server=requireNotNull(args.getString("migrationReceiverServerID"))
        val token=requireNotNull(args.getString("migrationReceiverToken"))
        val source=JSONObject(String(java.util.Base64.getDecoder().decode(args.getString("migrationProfileBase64")),Charsets.UTF_8))
        val profile=ControllerProfile.parse(source)
        val before=LayoutSnapshot.resolved(profile,LayoutOrientation.LANDSCAPE)
        val control=profile.controls(true).first().id
        val moved=LayoutControlPatch(control,mapOf("centerX" to .68)).applying(before)
        val pending=PendingLayoutEdit.create(profile.id,LayoutOrientation.LANDSCAPE,moved.json(),System.currentTimeMillis(),server)
        val store=MemorySessionStore()
        store.saveMac(TrustedMac(server,token,listOf(url!!)))
        store.saveEditingState(ProfileSnapshot(listOf(profile),profile.id,profile.id,serverID=server),emptyList(),emptyList(),listOf(pending))
        val state=AtomicReference(ClientState())
        // Exercise production admission, not an opt-in test constructor.
        val client=ControllerClient(store) { state.set(it) }
        try {
            client.resume()
            try {
                await("legacy edit acknowledged and durably drained",30000) {
                    state.get().paired && store.loadPendingLayoutEdits().isEmpty() && state.get().pendingLayoutChanges==0
                }
            } catch(error: Throwable) {
                println("LEGACY_PENDING_DIAGNOSTIC="+pending.customization.json())
                throw AssertionError("Legacy drain: paired=${state.get().paired}, error=${state.get().error}, pending=${store.loadPendingLayoutEdits().size}",error)
            }
            assertFalse(state.get().snapshot.selected!!.isOwned)
            assertEquals(.68,state.get().snapshot.selected!!.controls(true).first { it.id==control }.style.getDouble("centerX"),.00001)
            // The next authenticated handshake proves all legacy journals are
            // empty. The driver checks it before stopping/migrating this owner.
            android.os.SystemClock.sleep(1000) // Let the independent driver observe the authenticated legacy offer.
            client.suspend();client.resume()
            await("same trusted receiver rejoined after owned migration",60000) {
                state.get().paired && state.get().snapshot.selected?.isOwned==true
            }
            assertEquals(profile.id,state.get().snapshot.selectedID)
            assertEquals(.68,state.get().snapshot.selected!!.controls(true).first { it.id==control }.style.getDouble("centerX"),.00001)
            assertTrue(store.loadPendingLayoutEdits().isEmpty())
            assertTrue(store.loadPendingSkinSelections().isEmpty())
            assertTrue(store.loadPendingAppearancePreferences().isEmpty())
            assertTrue(store.loadPendingSkinRemovals().isEmpty())
            val revision=state.get().snapshot.configurationRevision!!
            client.setAppearancePreference(AppearancePreference.DARK)
            await("production owned edit committed after migration",30000) {
                state.get().snapshot.configurationRevision==revision+BigInteger.ONE && state.get().pendingLayoutChanges==0 &&
                    state.get().snapshot.selected!!.layout(true).optString("colorSchemePreference")=="dark"
            }
            assertTrue(store.loadPendingOwnedPhoneEdits().isEmpty())
        } finally { client.dispose() }
    }

    @Test fun pendingSkinRemovalPreventsOwnedAdmission() {
        val profile=profile();val store=MemorySessionStore();val state=AtomicReference(ClientState())
        val removal=PendingSkinRemoval(SkinReference("com.thumble.fixture","1.0.0"),"mac-a")
        store.savePendingSkinRemovals(listOf(removal))
        Peer { ws,message -> if(message.optString("type") in listOf("hello","pairing_request")) ws.send(snapshot(profile,41,"pairing_accepted").toString()) }.use { peer ->
            val client=ControllerClient(store) { state.set(it) }
            try {
                client.connect(peer.target())
                await("pending removals block admission") { peer.received.isNotEmpty() && state.get().error?.contains("unsupported profile")==true }
                assertEquals("[4]",peer.received.first().getJSONArray("supportedProfileFormats").toString())
                assertEquals(listOf(removal),store.loadPendingSkinRemovals())
                assertFalse(state.get().paired)
            } finally { client.dispose() }
        }
    }

    @Test fun physicalClientUsesRealReceiverAndNativeHelper() {
        val args=InstrumentationRegistry.getArguments()
        val url=args.getString("ownedReceiverURL")
        org.junit.Assume.assumeTrue("Run through verify-owned-phone-android.py",url!=null)
        val server=requireNotNull(args.getString("ownedReceiverServerID"))
        val token=requireNotNull(args.getString("ownedReceiverToken"))
        val second=UUID.fromString(args.getString("ownedReceiverSecondProfileID"))
        val store=MemorySessionStore()
        store.saveMac(TrustedMac(server,token,listOf(url!!)))
        val state=AtomicReference(ClientState())
        val client=ControllerClient(store) { state.set(it) }
        try {
            client.resume()
            await("real receiver paired") { state.get().paired }
            val initial=state.get().snapshot
            val profile=initial.selected!!
            val revision=initial.configurationRevision!!
            assertTrue(profile.isOwned);assertEquals(args.getString("ownedReceiverProfileCount")!!.toInt(),initial.profiles.size)
            // The receiver sends a typed display projection. Its opaque decimal
            // arrives as 1.0; raw source precision is checked on the Mac. The
            // phone must retain the token it actually received and never send
            // its profile projection back as replacement authority.
            assertEquals("1.0",profile.source.get("phonePrecise").toString())
            val bank=profile.source.getJSONObject("appearanceBank").toString()
            client.beginLayoutEditing(profile.id,LayoutOrientation.LANDSCAPE)
            await("real receiver editor") { state.get().layoutEditing }
            val editing=state.get()
            val control=profile.controls(true).first { !it.style.optBoolean("isLocationLocked") }
            val target=if(control.style.optDouble("centerX",.5)==.68) .66 else .68
            val moved=LayoutControlPatch(control.id,mapOf("centerX" to target)).applying(editing.layoutEditEntry!!)
            client.commitLayoutEdit(moved,editing.layoutEditToken)
            await("real receiver committed and acknowledged geometry") {
                state.get().snapshot.configurationRevision==revision+BigInteger.ONE && state.get().pendingLayoutChanges==0
            }
            assertTrue(store.loadPendingOwnedPhoneEdits().isEmpty())
            assertEquals(bank,state.get().snapshot.selected!!.source.getJSONObject("appearanceBank").toString())
            assertEquals(target,state.get().snapshot.selected!!.controls(true).first { it.id==control.id }.style.getDouble("centerX"),0.0)
            client.finishLayoutEditing()
            await("editor closed") { !state.get().layoutEditing }
            val preference=if(AppearancePreference.forProfile(state.get().snapshot.selected!!)==AppearancePreference.DARK) AppearancePreference.LIGHT else AppearancePreference.DARK
            client.setAppearancePreference(preference)
            await("real receiver committed and acknowledged appearance") {
                state.get().snapshot.configurationRevision==revision+BigInteger.valueOf(2) && state.get().pendingLayoutChanges==0
            }
            assertEquals(preference,AppearancePreference.forProfile(state.get().snapshot.selected!!))
            client.selectProfile(second)
            await("real receiver selected B") { state.get().snapshot.selectedID==second }
            client.selectProfile(profile.id)
            await("real receiver restored A") { state.get().snapshot.selectedID==profile.id }
            assertEquals(initial.defaultID,state.get().snapshot.defaultID)
            assertEquals(target,state.get().snapshot.selected!!.controls(true).first { it.id==control.id }.style.getDouble("centerX"),0.0)
        } finally { client.dispose() }
    }

}
