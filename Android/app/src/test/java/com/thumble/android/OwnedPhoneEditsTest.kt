package com.thumble.android

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import java.math.BigInteger
import java.util.UUID

class OwnedPhoneEditsTest {
    private fun data(path: String) = javaClass.classLoader!!.getResourceAsStream(path)!!.use { it.readBytes() }
    private fun profileJSON(): String = JSONObject(data("owned-configuration/v1/document.json").toString(Charsets.UTF_8)).getJSONArray("profiles").getJSONObject(0).toString()
    private fun edit(id: UUID=UUID.randomUUID()): OwnedPhoneEdit {
        val raw=JSONObject(data("owned-phone-edits/v1/intent.json").toString(Charsets.UTF_8))
        raw.put("editID",id.toString()).put("profileID",JSONObject(profileJSON()).getString("id"))
        return OwnedPhoneEdit.decode(raw.toString().toByteArray())
    }
    @Test fun sharedRustIntentCasesMatchExactlyAndRemainDetached() {
        val cases=JSONObject(data("owned-phone-edits/v1/cases.json").toString(Charsets.UTF_8)).getJSONArray("cases")
        repeat(cases.length()) { index ->
            val row=cases.getJSONObject(index);val bytes=(if(row.has("raw")) row.getString("raw") else row.getJSONObject("request").toString()).toByteArray()
            assertEquals(row.getString("name"),row.getBoolean("allowed"),runCatching { OwnedPhoneEdit.decode(bytes) }.isSuccess)
        }
        val value=edit();val before=value.data();value.json().put("colorScheme","light")
        assertArrayEquals(before,value.data())
    }
    @Test fun unsignedRevisionAndIntegerTokenKindsAreExact() {
        val raw=edit().json().put("baseConfigurationRevision",OwnedPhoneEdit.MAX_REVISION)
        assertEquals(OwnedPhoneEdit.MAX_REVISION,OwnedPhoneEdit.decode(raw.toString().toByteArray()).baseConfigurationRevision)
        raw.put("baseConfigurationRevision",OwnedPhoneEdit.MAX_REVISION+BigInteger.ONE)
        assertTrue(runCatching { OwnedPhoneEdit.decode(raw.toString().toByteArray()) }.isFailure)
        val text=edit().data().toString(Charsets.UTF_8)
        for(token in listOf("41.0","41e0","-1","18446744073709551616")) {
            assertTrue(token,runCatching { OwnedPhoneEdit.decode(text.replace(Regex("\\\"baseConfigurationRevision\\\"\\s*:\\s*41"),"\"baseConfigurationRevision\":$token").toByteArray()) }.isFailure)
        }
    }
    @Test fun geometryDiffCannotSmugglePaintOutputsResourcesOrCanvas() {
        val source=OwnedConfigurationDocument.decode(data("owned-configuration/v1/document.json")).projectedProfiles().first()
        val before=LayoutSnapshot.resolved(source,LayoutOrientation.LANDSCAPE)
        val target=source.controls(true).first()
        val after=LayoutControlPatch(target.id,mapOf("centerX" to .61,"rotationDegrees" to 12.0)).applying(before)
        val intent=OwnedPhoneEdit.between(source.id,before,after,BigInteger.valueOf(41))!!
        assertTrue(intent.applying(before).samePresentation(after))
        assertFalse(intent.data().toString(Charsets.UTF_8).contains("appearanceResources"))
        val paint=after.json();paint.put("backgroundLightColor",JSONObject().put("red",.1).put("green",.2).put("blue",.3).put("alpha",1))
        assertTrue(runCatching { OwnedPhoneEdit.between(source.id,before,LayoutSnapshot.capture(paint),BigInteger.valueOf(41)) }.isFailure)
        assertNull(OwnedPhoneEdit.between(source.id,before,before.stamped(555),BigInteger.valueOf(41)))
    }
    @Test fun journalSurvivesRestartKeepsRequestsExactAndRequiresTheMatchingMacReceipt() {
        val first=PendingOwnedPhoneEdit.capture("mac-a",edit(),100,profileJSON())
        val second=PendingOwnedPhoneEdit.capture("mac-a",edit(),101,profileJSON())
        val other=PendingOwnedPhoneEdit.capture("mac-b",edit(),102,profileJSON())
        val values=listOf(first,second,other)
        val restored=OwnedPhoneEditJournal.decode(OwnedPhoneEditJournal.encode(values))
        assertArrayEquals(first.edit.data(),restored[0].edit.data())
        assertEquals(1,OwnedPhoneEditJournal.uploads(restored,"mac-a").size)
        val receipt=JSONObject().put("schemaVersion",1).put("editID",first.edit.editID.toString()).put("profileID",first.edit.profileID.toString())
            .put("baseConfigurationRevision",41).put("configurationRevision",42).put("committedRevision",42).put("changed",true).put("status","applied")
        assertEquals(3,OwnedPhoneEditJournal.acknowledge(restored,"mac-b",receipt).size)
        assertEquals(2,OwnedPhoneEditJournal.acknowledge(restored,"mac-a",receipt).size)
        receipt.put("status","conflict")
        assertEquals(3,OwnedPhoneEditJournal.acknowledge(restored,"mac-a",receipt).size)
        receipt.put("status","replayed")
        assertEquals(2,OwnedPhoneEditJournal.acknowledge(restored,"mac-a",receipt).size)
        receipt.put("committedRevision",40)
        assertTrue(runCatching { OwnedPhoneEditJournal.acknowledge(restored,"mac-a",receipt) }.isFailure)
        val encoded=JSONObject(OwnedPhoneEditJournal.encode(values).toString(Charsets.UTF_8))
        assertEquals(1,encoded.getJSONObject("recoveryProfiles").length())
    }
    @Test fun byteBoundsAndCorruptRecoveryNeverProducePartialResults() {
        val original=edit().data()
        val padded=original+ByteArray(OwnedPhoneEdit.MAX_BYTES-original.size) { 32 }
        assertArrayEquals(padded,OwnedPhoneEdit.decode(padded).data())
        assertTrue(runCatching { OwnedPhoneEdit.decode(padded+byteArrayOf(32)) }.isFailure)
        val pending=PendingOwnedPhoneEdit.capture("mac-a",edit(),100,profileJSON())
        val encoded=JSONObject(OwnedPhoneEditJournal.encode(listOf(pending)).toString(Charsets.UTF_8))
        val table=encoded.getJSONObject("recoveryProfiles")
        val key=table.keys().next()
        table.put(key,JSONObject(table.getString(key)).put("name","Tampered recovery").toString())
        assertTrue(runCatching { OwnedPhoneEditJournal.decode(encoded.toString().toByteArray()) }.isFailure)
    }
    @Test fun memoryStoreKeepsOwnedEditsAcrossLegacySavesAndCredentialRetirement() {
        val store=MemorySessionStore()
        val pending=PendingOwnedPhoneEdit.capture("mac-a",edit(),100,profileJSON())
        val snapshot=ProfileSnapshot()
        store.saveOwnedEditingState(snapshot,emptyList(),emptyList(),emptyList(),listOf(pending))
        store.saveSnapshot(snapshot)
        store.saveSkinState(snapshot,emptyList())
        store.saveProfileState(snapshot,emptyList(),emptyList())
        store.saveEditingState(snapshot,emptyList(),emptyList(),emptyList())
        store.clearMacCredential()
        assertArrayEquals(pending.edit.data(),store.loadPendingOwnedPhoneEdits().single().edit.data())
        store.forgetMac()
        assertTrue(store.loadPendingOwnedPhoneEdits().isEmpty())
    }

    @Test fun ownedProfileCacheRetainsCanonicalSourceAndExactRevision() {
        val raw=JSONObject(profileJSON()).put("futureProfileMetadata",JSONObject().put("keep",true))
        val profile=ControllerProfile.parse(raw)
        assertTrue(profile.isOwned)
        assertEquals("landscape",profile.orientationPreference)
        assertTrue(profile.source.getJSONObject("futureProfileMetadata").getBoolean("keep"))
        assertFalse(profile.source.has("orientationPreference"))
        assertFalse(profile.source.has("landscapeCustomization"))
        assertEquals(5,profile.source.getInt("profileFormatVersion"))
        val message=WireCodec.message("gamepad_profiles").put("serverID","mac-a")
            .put("gamepadProfiles",org.json.JSONArray().put(raw)).put("gamepadProfileID",profile.id.toString())
            .put("configurationRevision",OwnedPhoneEdit.MAX_REVISION)
        val snapshot=ProfileSnapshot().applying(WireCodec.decode(message.toString().toByteArray(),exactNumbers=true))
        assertEquals(OwnedPhoneEdit.MAX_REVISION,snapshot.configurationRevision)
        assertEquals(OwnedPhoneEdit.MAX_REVISION,snapshot.applying(WireCodec.message("gamepad_profiles")).configurationRevision)
        assertNull(snapshot.applying(WireCodec.message("gamepad_profiles").put("serverID","mac-b"),replaceServer=true).configurationRevision)
        assertTrue(runCatching { ControllerProfile.parse(JSONObject(raw.toString()).put("portraitCustomization",raw.getJSONObject("customization"))) }.isFailure)
        assertTrue(runCatching { WireCodec.decode("{\"configurationRevision\":1e9999}".toByteArray(),exactNumbers=true) }.isFailure)
        // Wire activation remains a separate admission gate until journals and
        // actual receiver receipt handling are connected and verified.
        assertEquals(listOf(4),ProfileFramePolicy.supportedFormats)
    }

    @Test fun conflictRetryNeedsExplicitReceiptAndFreshAuthorityAndRetainsLaterIntents() {
        val profile=ControllerProfile.parse(JSONObject(profileJSON()))
        val before=LayoutSnapshot.resolved(profile,LayoutOrientation.LANDSCAPE)
        val control=profile.controls(true).first().id
        val moved=LayoutControlPatch(control,mapOf("centerX" to .62)).applying(before)
        val resized=LayoutControlPatch(control,mapOf("widthScale" to 1.7)).applying(moved)
        val first=OwnedPhoneEdit.between(profile.id,before,moved,BigInteger.valueOf(41))!!
        val second=OwnedPhoneEdit.between(profile.id,moved,resized,BigInteger.valueOf(41))!!
        val recovery=second.applying(first.applying(profile,100),101)
        val pending=listOf(first,second).mapIndexed { index, edit ->
            PendingOwnedPhoneEdit.capture("mac-a",edit,100L+index,recovery.source.toString())
        }
        val original=pending.map { it.edit.data() }
        val receipt=JSONObject().put("schemaVersion",1).put("editID",first.editID.toString()).put("profileID",profile.id.toString())
            .put("baseConfigurationRevision",41).put("configurationRevision",42).put("changed",false).put("status","conflict")
            .put("errorCode","owned_phone_revision_conflict")
        val renamed=ControllerProfile.parse(JSONObject(profile.source.toString()).put("name","Renamed on Mac").put("futureSyncMetadata","keep"))
        val fresh=ProfileSnapshot(listOf(renamed),renamed.id,serverID="mac-a",configurationRevision=BigInteger.valueOf(42))
        assertEquals(pending,OwnedPhoneEditJournal.retryAfterConflict(pending,"mac-b",receipt,fresh))
        assertTrue(runCatching { OwnedPhoneEditJournal.retryAfterConflict(pending,"mac-a",receipt,fresh.copy(configurationRevision=BigInteger.valueOf(41))) }.isFailure)
        val retried=OwnedPhoneEditJournal.retryAfterConflict(pending,"mac-a",receipt,fresh)
        assertNotEquals(first.editID,retried[0].edit.editID)
        assertEquals(BigInteger.valueOf(42),retried[0].edit.baseConfigurationRevision)
        assertArrayEquals(original[1],retried[1].edit.data())
        pending.forEachIndexed { index, value -> assertArrayEquals(original[index],value.edit.data()) }
        val saved=ControllerProfile.parse(JSONObject(retried[0].recoveryProfileJSON()))
        assertEquals("Renamed on Mac",saved.name)
        assertEquals("keep",saved.source.getString("futureSyncMetadata"))
        assertEquals(profile.source.getJSONObject("appearanceBank").toString(),saved.source.getJSONObject("appearanceBank").toString())
        assertTrue(LayoutSnapshot.resolved(saved,LayoutOrientation.LANDSCAPE).samePresentation(resized))
        // A separate authority import can remove an owner entirely; ordinary
        // preset deletion uses native hide semantics and retains that owner.
        val absent=StrictJSON.objectCopy(renamed.source)
        absent.put("customization",LayoutControlPatch(control,delete=true).applying(before,deleteDeclaredElement=true).json())
        absent.getJSONObject("appearanceBank").getJSONArray("variants").let { variants -> repeat(variants.length()) {
            variants.getJSONObject(it).getJSONObject("appearance").getJSONObject("controls").remove(control.toString())
        } }
        val deleted=ControllerProfile.parse(absent)
        assertTrue(runCatching { OwnedPhoneEditJournal.retryAfterConflict(pending,"mac-a",receipt,fresh.copy(profiles=listOf(deleted))) }.isFailure)
        receipt.put("status","rejected")
        assertTrue(runCatching { OwnedPhoneEditJournal.retryAfterConflict(pending,"mac-a",receipt,fresh) }.isFailure)
    }

    @Test fun exactJSONWriterDoesNotRoundOpaqueNumbersToNearbyIntegers() {
        val raw="{\"nearInteger\":1.0000000000000000000001,\"largeInteger\":9007199254740993,\"maximumRevision\":18446744073709551615}"
        val parsed=StrictJSON.objectValue(raw.toByteArray(),exactNumbers=true)
        val copy=StrictJSON.objectValue(StrictJSON.data(parsed),exactNumbers=true)
        assertEquals("1.0000000000000000000001",copy.get("nearInteger").toString())
        assertEquals("9007199254740993",copy.get("largeInteger").toString())
        assertEquals("18446744073709551615",copy.get("maximumRevision").toString())
        val source=StrictJSON.objectValue((profileJSON().dropLast(1)+",\"phonePrecise\":1.0000000000000000000001}").toByteArray(),exactNumbers=true)
        val profile=ControllerProfile.parse(source)
        assertEquals("1.0000000000000000000001",profile.source.get("phonePrecise").toString())
        val before=LayoutSnapshot.resolved(profile,LayoutOrientation.LANDSCAPE)
        val moved=LayoutControlPatch(profile.controls(true).first().id,mapOf("centerX" to .65)).applying(before)
        val intent=OwnedPhoneEdit.between(profile.id,before,moved,BigInteger.valueOf(41))!!
        assertEquals("1.0000000000000000000001",intent.applying(profile,100).source.get("phonePrecise").toString())
    }

    @Test fun ownedFrameAdmissionRequiresRevisionAndCompleteExplicitOwnedProfiles() {
        val source=ControllerProfile.parse(JSONObject(profileJSON()))
        val frame=WireCodec.message("gamepad_profiles").put("profileFormatVersion",5).put("configurationRevision",41)
            .put("gamepadProfiles",org.json.JSONArray().put(source.source))
        ProfileFramePolicy.validateIncoming(frame,listOf(4,5))
        assertTrue(runCatching { ProfileFramePolicy.validateIncoming(JSONObject(frame.toString()).also { it.remove("configurationRevision") },listOf(4,5)) }.isFailure)
        assertTrue(runCatching { ProfileFramePolicy.validateIncoming(JSONObject(frame.toString()).put("configurationRevision",41.0),listOf(4,5)) }.isFailure)
        val scalar=WireCodec.message("gamepad_customization").put("profileFormatVersion",5).put("configurationRevision",41)
            .put("gamepadCustomization",source.source.getJSONObject("customization"))
        assertTrue(runCatching { ProfileFramePolicy.validateIncoming(scalar,listOf(4,5)) }.isFailure)
        val missingMarker=JSONObject(frame.toString())
        missingMarker.getJSONArray("gamepadProfiles").getJSONObject(0).remove("profileFormatVersion")
        assertTrue(runCatching { ProfileFramePolicy.validateIncoming(missingMarker,listOf(4,5)) }.isFailure)
    }

    @Test fun removedPresetUsesNativeHideAndKeepsItsAppearanceOwner() {
        val profile=ControllerProfile.parse(JSONObject(profileJSON()))
        val owner=profile.controls(true).first().id
        val intent=OwnedPhoneEdit.decode(JSONObject().put("schemaVersion",1).put("editID",UUID.randomUUID().toString()).put("profileID",profile.id.toString())
            .put("baseConfigurationRevision",41).put("removedControlIDs",org.json.JSONArray().put(owner.toString())).toString().toByteArray())
        val preview=intent.applying(profile,100)
        val element=AppearanceJSON.objects(preview.source.getJSONObject("customization").getJSONArray("elements")).single { UUID.fromString(it.getString("id"))==owner }
        assertTrue(element.getJSONObject("layout").getBoolean("isHidden"))
        assertEquals(profile.source.getJSONObject("appearanceBank").toString(),preview.source.getJSONObject("appearanceBank").toString())
        assertTrue(preview.controls(true).none { it.id==owner })
        val absent=LayoutControlPatch(owner,delete=true).applying(LayoutSnapshot.resolved(profile,LayoutOrientation.LANDSCAPE),deleteDeclaredElement=true)
        assertTrue(runCatching { intent.applying(absent) }.isFailure)
    }

}
