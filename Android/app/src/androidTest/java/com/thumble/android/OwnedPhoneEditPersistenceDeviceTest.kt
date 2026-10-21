package com.thumble.android

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.util.UUID

@RunWith(AndroidJUnit4::class)
class OwnedPhoneEditPersistenceDeviceTest {
    private val instrumentation get()=InstrumentationRegistry.getInstrumentation()
    private fun asset(path: String)=instrumentation.context.assets.open(path).bufferedReader().use { it.readText() }
    private fun withStore(body: (AndroidSessionStore,File,PendingOwnedPhoneEdit)->Unit) {
        val namespace="owned-edit-test-${UUID.randomUUID()}"
        val directory=File(instrumentation.targetContext.noBackupFilesDir,namespace)
        val store=AndroidSessionStore(instrumentation.targetContext,namespace)
        val profile=JSONObject(asset("owned-configuration/v1/document.json")).getJSONArray("profiles").getJSONObject(0)
        val intent=JSONObject(asset("owned-phone-edits/v1/intent.json")).put("profileID",profile.getString("id"))
        val pending=PendingOwnedPhoneEdit.capture("mac-a",OwnedPhoneEdit.decode(intent.toString().toByteArray()),100,profile.toString())
        try { body(store,directory,pending) }
        finally { directory.setWritable(true,true);File(directory,"profiles.json").setWritable(true,true);store.forgetMac();directory.deleteRecursively() }
    }
    @Test fun atomicCacheKeepsExactIntentAndRecoveryAcrossReloadAndLegacySaves()=withStore { store,directory,pending ->
        val profile=ControllerProfile.parse(JSONObject(pending.recoveryProfileJSON()))
        val snapshot=ProfileSnapshot(listOf(profile),profile.id,profile.id,serverID="mac-a",configurationRevision=OwnedPhoneEdit.MAX_REVISION)
        store.saveOwnedEditingState(snapshot,emptyList(),emptyList(),emptyList(),listOf(pending))
        val reloaded=AndroidSessionStore(instrumentation.targetContext,directory.name)
        reloaded.saveSnapshot(snapshot)
        reloaded.saveSkinState(snapshot,emptyList())
        reloaded.saveProfileState(snapshot,emptyList(),emptyList())
        reloaded.saveEditingState(snapshot,emptyList(),emptyList(),emptyList())
        reloaded.clearMacCredential()
        assertEquals(OwnedPhoneEdit.MAX_REVISION,reloaded.loadSnapshot().configurationRevision)
        val cached=reloaded.loadSnapshot().selected!!
        assertTrue(cached.isOwned)
        assertEquals(JSONObject(pending.recoveryProfileJSON()).getInt("profileFormatVersion"),cached.source.getInt("profileFormatVersion"))
        assertFalse(cached.source.has("portraitCustomization"))
        assertFalse(cached.source.has("orientationPreference"))
        val saved=reloaded.loadPendingOwnedPhoneEdits().single()
        assertArrayEquals(pending.edit.data(),saved.edit.data())
        assertEquals(pending.recoveryProfileJSON(),saved.recoveryProfileJSON())
        assertEquals("mac-a",saved.serverID)
        reloaded.forgetMac()
        assertTrue(reloaded.loadPendingOwnedPhoneEdits().isEmpty())
    }
    @Test fun corruptJournalBlocksLegacyWritesWithoutErasingRecovery()=withStore { store,directory,pending ->
        store.saveOwnedEditingState(ProfileSnapshot(),emptyList(),emptyList(),emptyList(),listOf(pending))
        val file=File(directory,"profiles.json")
        val original=file.readText()
        val raw=JSONObject(original)
        raw.getJSONObject("androidPendingOwnedPhoneEdits").put("version",99)
        for(corrupt in listOf(raw.toString(),original.replace("\"version\":1","\"version\":1.0"),original.replace("\"updatedAt\":100","\"updatedAt\":100.0"))) {
            file.writeText(corrupt)
            val before=file.readBytes()
            assertThrows(Exception::class.java) { store.loadPendingOwnedPhoneEdits() }
            assertThrows(Exception::class.java) { store.saveSnapshot(ProfileSnapshot()) }
            assertArrayEquals(before,file.readBytes())
        }
    }
    @Test fun failedAtomicWriteKeepsPreviousIntentBytes()=withStore { store,directory,pending ->
        store.saveOwnedEditingState(ProfileSnapshot(),emptyList(),emptyList(),emptyList(),listOf(pending))
        val file=File(directory,"profiles.json");val before=file.readBytes()
        assertTrue(file.setWritable(false,false));assertTrue(directory.setWritable(false,false))
        try { assertThrows(Exception::class.java) { store.saveOwnedEditingState(ProfileSnapshot(),emptyList(),emptyList(),emptyList(),emptyList()) } }
        finally { directory.setWritable(true,true);file.setWritable(true,true) }
        assertArrayEquals(before,file.readBytes())
        assertArrayEquals(pending.edit.data(),store.loadPendingOwnedPhoneEdits().single().edit.data())
    }
    private fun await(label: String, predicate: ()->Boolean) {
        val until=android.os.SystemClock.elapsedRealtime()+5000
        while(!predicate() && android.os.SystemClock.elapsedRealtime()<until) android.os.SystemClock.sleep(10)
        assertTrue(label,predicate())
    }
    @Test fun clientRestoresPendingCountAndDoesNotDiscardTheJournalOffline()=withStore { store,_,pending ->
        val profile=ControllerProfile.parse(JSONObject(pending.recoveryProfileJSON()))
        val snapshot=ProfileSnapshot(listOf(profile),profile.id,profile.id,serverID="mac-a",configurationRevision=OwnedPhoneEdit.MAX_REVISION)
        store.saveOwnedEditingState(snapshot,emptyList(),emptyList(),emptyList(),listOf(pending))
        val state=java.util.concurrent.atomic.AtomicReference(ClientState())
        val client=ControllerClient(store) { state.set(it) }
        try {
            await("owned edit restored") { state.get().onboardingLoaded && state.get().pendingLayoutChanges==1 }
            client.viewSavedKeypad()
            await("offline cache remains available") { state.get().viewingSaved }
            assertTrue(state.get().snapshot.selected!!.isOwned)
            assertArrayEquals(pending.edit.data(),store.loadPendingOwnedPhoneEdits().single().edit.data())
            assertFalse(state.get().paired)
            assertEquals(0L,state.get().webSocketFrames)
        } finally { client.dispose() }
    }

}
