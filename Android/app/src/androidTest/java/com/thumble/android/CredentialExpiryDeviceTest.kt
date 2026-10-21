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
import java.io.File
import java.net.InetAddress
import java.util.UUID
import java.util.concurrent.atomic.AtomicReference

@RunWith(AndroidJUnit4::class)
class CredentialExpiryDeviceTest {
    private val context get()=InstrumentationRegistry.getInstrumentation().targetContext
    private fun profile()=ControllerProfile.parse(JSONObject(InstrumentationRegistry.getInstrumentation().context.assets
        .open("state/minimal-profile.json").bufferedReader().use { it.readText() }))
    private fun await(label: String,predicate: ()->Boolean) {
        val deadline=android.os.SystemClock.elapsedRealtime()+8000
        while(!predicate() && android.os.SystemClock.elapsedRealtime()<deadline) android.os.SystemClock.sleep(10)
        assertTrue(label,predicate())
    }
    @Test fun expiryKeepsProfilesAndEveryPendingJournalAcrossProcessReload() {
        val namespace="expiry-test-${UUID.randomUUID()}"
        val server=MockWebServer();server.start(InetAddress.getByName("127.0.0.1"),0)
        server.enqueue(MockResponse().withWebSocketUpgrade(object: WebSocketListener() {
            override fun onMessage(ws: WebSocket,text: String) { if(JSONObject(text).optString("type")=="hello")
                ws.send(WireCodec.message("error").put("message","Trusted pairing expired. Request pairing again.").toString()) }
        }))
        val store=AndroidSessionStore(context,namespace)
        val profile=profile();val snapshot=ProfileSnapshot(listOf(profile),profile.id,profile.id)
        val skin=PendingSkinSelection(profile.id,SkinReference("com.example.expiry","1.0.0"),100,"mac-a")
        val appearance=PendingAppearancePreference.create(profile,AppearancePreference.DARK,101,"mac-a")
        val layout=PendingLayoutEdit.create(profile.id,LayoutOrientation.PORTRAIT,profile.layout(false),102,"mac-a")
        val removal=PendingSkinRemoval(SkinReference("com.example.unused","1.0.0"),"mac-a")
        store.saveEditingState(snapshot,listOf(skin),listOf(appearance),listOf(layout));store.savePendingSkinRemovals(listOf(removal))
        store.saveOnboardingCompleted();store.saveMac(TrustedMac("mac-a","revoked-test-only",listOf("ws://127.0.0.1:${server.port}")))
        val state=AtomicReference(ClientState());var client: ControllerClient?=null
        try {
            client=ControllerClient(store) { state.set(it) };client.resume()
            await("expiry handled") { state.get().error?.contains("expired")==true }
            assertNull(store.loadMac());assertFalse(state.get().canReconnect)
            val reloaded=AndroidSessionStore(context,namespace)
            assertEquals("cached profile must survive credential expiry",profile.id,reloaded.loadSnapshot().selectedID)
            assertEquals("mac-a",reloaded.loadSnapshot().serverID)
            assertEquals(listOf(skin),reloaded.loadPendingSkinSelections())
            assertEquals(listOf(appearance),reloaded.loadPendingAppearancePreferences())
            assertEquals(listOf(layout),reloaded.loadPendingLayoutEdits())
            assertEquals(listOf(removal),reloaded.loadPendingSkinRemovals())
            assertTrue(reloaded.loadOnboardingCompleted())
            client.dispose();client=null
            val restored=AtomicReference(ClientState());client=ControllerClient(reloaded) { restored.set(it) }
            await("new process restores saved profile") { restored.get().snapshot.selectedID==profile.id }
            client.viewSavedKeypad();await("offline practice available") { restored.get().viewingSaved }
            assertFalse(restored.get().paired)
            assertEquals(0L,restored.get().webSocketFrames)
            client.forgetMac();await("explicit forget") { restored.get().status=="Mac forgotten" }
            assertTrue(reloaded.loadSnapshot().profiles.isEmpty());assertTrue(reloaded.loadPendingSkinSelections().isEmpty())
        } finally { client?.dispose();server.shutdown();File(context.noBackupFilesDir,namespace).deleteRecursively() }
    }
    @Test fun freshPairingKeepsOldMacEditsIsolatedAndReturningToThatMacReplaysThem() {
        val serverA=MockWebServer();val serverB=MockWebServer()
        serverA.start(InetAddress.getByName("127.0.0.1"),0);serverB.start(InetAddress.getByName("127.0.0.1"),0)
        val original=profile()
        val cached=ControllerProfile.parse(JSONObject(original.source.toString()).put("customization",JSONObject(original.layout(false).toString()).put("showsButtonLabels",false)))
        val store=MemorySessionStore()
        store.saveSnapshot(ProfileSnapshot(listOf(cached),cached.id,cached.id,serverID="mac-a"))
        store.saveMac(TrustedMac("mac-a","revoked-test-only",listOf("ws://127.0.0.1:${serverA.port}")))
        val skin=SkinReference("com.example.expiry","1.0.0")
        store.saveEditingState(store.loadSnapshot(),listOf(PendingSkinSelection(cached.id,skin,100,"mac-a")),
            listOf(PendingAppearancePreference.create(cached,AppearancePreference.DARK,101,"mac-a")),
            listOf(PendingLayoutEdit.create(cached.id,LayoutOrientation.PORTRAIT,cached.layout(false),102,"mac-a")))
        store.savePendingSkinRemovals(listOf(PendingSkinRemoval(SkinReference("com.example.unused","1.0.0"),"mac-a")))
        serverA.enqueue(MockResponse().withWebSocketUpgrade(object:WebSocketListener() {
            override fun onMessage(ws:WebSocket,text:String) { if(JSONObject(text).optString("type")=="hello") ws.send(WireCodec.message("pairing_challenge").toString()) }
        }))
        val messagesA=java.util.concurrent.CopyOnWriteArrayList<JSONObject>()
        val messagesB=java.util.concurrent.CopyOnWriteArrayList<JSONObject>()
        fun acceptedListener(id: String,messages: MutableList<JSONObject>)=object:WebSocketListener() {
            val remote=JSONObject(original.source.toString())
            fun snapshot(type: String)=WireCodec.message(type).put("serverID",id).put("inputProtocolVersion",3)
                .put("authToken","fresh-fixture-only").put("gamepadProfiles",JSONArray().put(JSONObject(remote.toString())))
                .put("gamepadProfileID",original.id.toString()).put("capabilities",JSONArray(listOf("gamepad_profile_skin_selection",ProfileActions.ORIENTATION_CAPABILITY,"skin_packages")))
            override fun onMessage(ws:WebSocket,text:String) {
                val message=JSONObject(text);messages.add(message)
                when(message.optString("type")) {
                    "hello" -> { ws.send(snapshot("pairing_accepted").toString());ws.send(WireCodec.message("ping").put("timestamp",123).toString()) }
                    "gamepad_profile_skin_selection" -> {
                        remote.put("skinReference",message.optJSONObject("skinReference"));ws.send(snapshot("gamepad_profiles").toString())
                    }
                    "gamepad_customization" -> {
                        remote.put("customization",message.getJSONObject("gamepadCustomization"));ws.send(snapshot("gamepad_profiles").toString())
                    }
                }
            }
        }
        serverA.enqueue(MockResponse().withWebSocketUpgrade(acceptedListener("mac-a",messagesA)))
        serverB.enqueue(MockResponse().withWebSocketUpgrade(acceptedListener("mac-b",messagesB)))
        val state=AtomicReference(ClientState());val client=ControllerClient(store) { state.set(it) }
        try {
            client.resume();await("challenge retires trust") { !state.get().canReconnect && state.get().error?.contains("expired")==true }
            client.viewSavedKeypad();await("saved keypad available") { state.get().viewingSaved }
            client.setAppearancePreference(AppearancePreference.LIGHT)
            await("offline appearance retains cached ownership") { store.loadPendingAppearancePreferences().singleOrNull()?.let { it.serverID=="mac-a" && it.preference==AppearancePreference.LIGHT }==true }
            await("offline appearance published") { state.get().snapshot.selected?.let { AppearancePreference.forProfile(it)==AppearancePreference.LIGHT }==true }
            val previous=state.get().snapshot.selected!!
            val replacement=ControllerProfile.parse(JSONObject(previous.source.toString()).put("skinReference",JSONObject().put("identifier",skin.identifier).put("version",skin.version)))
            client.commitSkinChange(previous,replacement,skin)
            await("offline skin saved for cached owner") { store.loadPendingSkinSelections().singleOrNull()?.updatedAt?.let { it>100 }==true }
            assertEquals("mac-a",store.loadPendingSkinSelections().single().serverID)
            client.connect(PairingTarget.manual("ws://127.0.0.1:${serverB.port}","123456"))
            await("other Mac completes ordered ping barrier") { state.get().pairedServerID=="mac-b" && messagesB.any { it.optString("type")=="pong" } }
            assertFalse(messagesB.first { it.optString("type")=="hello" }.has("authToken"))
            assertTrue("no edits sent to the Mac with colliding profile UUIDs",messagesB.none { it.optString("type") in setOf("gamepad_customization","gamepad_profile_skin_selection","skin_package_removal","skin_packages") })
            assertEquals(1,store.loadPendingSkinSelections().size);assertEquals(1,store.loadPendingAppearancePreferences().size)
            assertEquals(1,store.loadPendingLayoutEdits().size);assertEquals(1,store.loadPendingSkinRemovals().size)
            client.connect(PairingTarget.manual("ws://127.0.0.1:${serverA.port}","123456"))
            await("original Mac receives its retained skin and customization") { state.get().pairedServerID=="mac-a" && messagesA.any { it.optString("type")=="gamepad_profile_skin_selection" } && messagesA.any { it.optString("type")=="gamepad_customization" } }
            await("original authority acknowledges retained changes") { store.loadPendingSkinSelections().isEmpty() && store.loadPendingAppearancePreferences().isEmpty() && store.loadPendingLayoutEdits().isEmpty() }
            assertFalse(messagesA.first { it.optString("type")=="hello" }.has("authToken"))
        } finally { client.dispose();serverA.shutdown();serverB.shutdown() }
    }

    @Test fun connectionScreenOffersPracticeAndExplicitForgetWithoutAValidCredential() {
        val profile=profile();val memory=MemorySessionStore().apply {
            saveSnapshot(ProfileSnapshot(listOf(profile),profile.id,profile.id,serverID="mac-a"))
        }
        val previous=ClientDependencies.sessionStore;ClientDependencies.sessionStore={memory}
        try {
            androidx.test.core.app.ActivityScenario.launch(MainActivity::class.java).use { scenario ->
                await("cached connection screen loaded") {
                    var ready=false
                    scenario.onActivity { ready=it.connectionState.onboardingLoaded && it.connectionState.snapshot.selectedID==profile.id && !it.connectionState.canReconnect }
                    ready
                }
                fun visible(label: String): Boolean {
                    fun walk(node: android.view.accessibility.AccessibilityNodeInfo): Boolean {
                        if(!node.refresh()) return false
                        if(node.isVisibleToUser && (node.text?.toString()==label || node.contentDescription?.toString()==label)) return true
                        return (0 until node.childCount).any { node.getChild(it)?.let(::walk)==true }
                    }
                    return InstrumentationRegistry.getInstrumentation().uiAutomation.rootInActiveWindow?.let(::walk)==true
                }
                await("saved keypad can still be practiced") { visible("Practice with saved keypad") }
                await("retained cache can be explicitly forgotten") { visible("Forget saved Mac") }
                assertNull(memory.loadMac());assertEquals(profile.id,memory.loadSnapshot().selectedID)
            }
        } finally { ClientDependencies.sessionStore=previous }
    }

}
