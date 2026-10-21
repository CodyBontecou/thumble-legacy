package com.thumble.android

import org.json.JSONArray
import org.json.JSONObject
import java.security.MessageDigest
import java.util.UUID

/** Intent bytes never change after transmission. Recovery data is local only. */
internal class PendingOwnedPhoneEdit private constructor(val serverID: String, val edit: OwnedPhoneEdit,
    val updatedAt: Long, internal val recovery: Recovery) {
    internal class Recovery(val source: String, val profileID: UUID) {
        val digest: String
        init {
            require(source.toByteArray().size<=WireCodec.MAX_PAYLOAD)
            // Validate the explicit v5 profile without flattening its raw bytes.
            val envelope="{\"schema\":\"com.codybontecou.pocketpad.keypad-configuration\",\"version\":5,\"activeProfileID\":\"$profileID\",\"defaultProfileID\":null,\"profiles\":[$source]}"
            val document=OwnedConfigurationDocument.decode(envelope.toByteArray())
            require(document.activeProfileID==profileID && document.projectedProfiles().size==1)
            digest=MessageDigest.getInstance("SHA-256").digest(source.toByteArray()).joinToString("") { "%02x".format(it.toInt() and 255) }
        }
    }
    init {
        require(serverID.isNotBlank() && serverID==serverID.trim() && serverID.length<=512)
        require(updatedAt>=0 && recovery.profileID==edit.profileID)
    }
    fun recoveryProfileJSON() = recovery.source
    internal fun replacingRecovery(value: Recovery) = PendingOwnedPhoneEdit(serverID,edit,updatedAt,value)
    override fun toString() = "PendingOwnedPhoneEdit([private recovery], editID=${edit.editID})"
    companion object {
        fun capture(serverID: String, edit: OwnedPhoneEdit, now: Long, recoveryProfileJSON: String) =
            PendingOwnedPhoneEdit(serverID,edit,now,Recovery(recoveryProfileJSON,edit.profileID))
        internal fun restored(serverID: String, edit: OwnedPhoneEdit, now: Long, recovery: Recovery) =
            PendingOwnedPhoneEdit(serverID,edit,now,recovery)
    }
}

internal object OwnedPhoneEditJournal {
    const val MAX_EDITS=512
    /** One recovery profile per latest local snapshot, not an image table per gesture. */
    fun encode(values: List<PendingOwnedPhoneEdit>): ByteArray {
        require(values.size<=MAX_EDITS)
        require(values.map { it.serverID to it.edit.editID }.distinct().size==values.size)
        // Escaping can increase the final size; reject an already oversized
        // lower bound before allocating the encoded recovery table.
        val recoveryBytes=values.distinctBy { it.recovery.digest }.sumOf { it.recoveryProfileJSON().toByteArray().size.toLong() }
        val intentBytes=values.sumOf { it.edit.data().size.toLong() }
        require(recoveryBytes+intentBytes<=WireCodec.MAX_PAYLOAD) { "Pending owned edits exceed the storage bound" }
        val recoveries=JSONObject();val records=JSONArray()
        values.forEach { value ->
            recoveries.put(value.recovery.digest,value.recoveryProfileJSON())
            records.put(JSONObject().put("serverID",value.serverID).put("intentJSON",value.edit.data().toString(Charsets.UTF_8))
                .put("updatedAt",value.updatedAt).put("recovery",value.recovery.digest))
        }
        return JSONObject().put("version",1).put("pending",records).put("recoveryProfiles",recoveries).toString().toByteArray()
            .also { require(it.size<=WireCodec.MAX_PAYLOAD) { "Pending owned edits exceed the storage bound" } }
    }
    fun decode(bytes: ByteArray): List<PendingOwnedPhoneEdit> {
        require(bytes.size<=WireCodec.MAX_PAYLOAD)
        val raw=StrictJSON.objectValue(bytes,exactNumbers=true)
        require(raw.keys().asSequence().toSet()==setOf("version","pending","recoveryProfiles"))
        require(raw.opt("version") is Number && raw.get("version").toString()=="1")
        val records=raw.getJSONArray("pending");require(records.length()<=MAX_EDITS)
        val table=raw.getJSONObject("recoveryProfiles");require(table.length()<=MAX_EDITS)
        val decoded=hashMapOf<String,PendingOwnedPhoneEdit.Recovery>()
        val result=(0 until records.length()).map { index ->
            val row=records.getJSONObject(index)
            require(row.keys().asSequence().toSet()==setOf("serverID","intentJSON","updatedAt","recovery"))
            val edit=OwnedPhoneEdit.decode(row.getString("intentJSON").toByteArray())
            val key=row.getString("recovery")
            val profile=decoded.getOrPut(key) { PendingOwnedPhoneEdit.Recovery(table.getString(key),edit.profileID).also { require(it.digest==key) } }
            val timestamp=(row.get("updatedAt") as? Number)?.toString() ?: error("Invalid edit timestamp")
            require(timestamp.matches(Regex("[0-9]+")))
            PendingOwnedPhoneEdit.restored(row.getString("serverID"),edit,timestamp.toLong(),profile)
        }
        require(table.keys().asSequence().toSet()==decoded.keys) { "Unreferenced recovery data" }
        encode(result)
        return result
    }
    fun recording(values: List<PendingOwnedPhoneEdit>, value: PendingOwnedPhoneEdit): List<PendingOwnedPhoneEdit> {
        val previous=values.firstOrNull { it.serverID==value.serverID && it.edit.editID==value.edit.editID }
        if(previous!=null) {
            require(previous.edit.data().contentEquals(value.edit.data())) { "Edit ID reused for another intent" }
            return values
        }
        // Fresh snapshots retain the latest cached metadata/resources, while all
        // queued intent IDs and request bytes remain unchanged for exact replay.
        return (values.map { if(it.serverID==value.serverID && it.edit.profileID==value.edit.profileID)
            it.replacingRecovery(value.recovery) else it }+value).also(::encode)
    }
    /** A full snapshot never acknowledges an intent. Only a matching explicit
     * successful receipt from the same trusted Mac can retire its journal entry. */
    fun acknowledge(values: List<PendingOwnedPhoneEdit>, serverID: String, receipt: JSONObject): List<PendingOwnedPhoneEdit> {
        val status=receipt.optString("status")
        require(status in setOf("applied","replayed","conflict","rejected"))
        require(receipt.opt("schemaVersion") is Number && receipt.get("schemaVersion").toString()=="1")
        val editID=OwnedPhoneEdit.id(receipt.opt("editID"));val profileID=OwnedPhoneEdit.id(receipt.opt("profileID"))
        val base=OwnedPhoneEdit.revision(receipt.opt("baseConfigurationRevision"))
        val current=OwnedPhoneEdit.revision(receipt.opt("configurationRevision"))
        if(status !in setOf("applied","replayed")) return values
        require(!receipt.has("errorCode") || receipt.isNull("errorCode"))
        val committed=OwnedPhoneEdit.revision(receipt.opt("committedRevision"))
        val changed=receipt.opt("changed") as? Boolean ?: error("Invalid edit result")
        require(committed>=base && current>=committed && changed==(committed>base))
        return values.filterNot { it.serverID==serverID && it.edit.editID==editID && it.edit.profileID==profileID && it.edit.baseConfigurationRevision==base }
    }

    /** Retry only after the same Mac explicitly reports that the original
     * attempt conflicted. An advancing full snapshot supplies the new base;
     * it never acknowledges or silently rewrites an uncertain request. */
    fun retryAfterConflict(values: List<PendingOwnedPhoneEdit>, serverID: String, receipt: JSONObject,
        authority: ProfileSnapshot, newEditID: UUID=UUID.randomUUID()): List<PendingOwnedPhoneEdit> {
        acknowledge(values,serverID,receipt) // Validate the receipt envelope first.
        require(receipt.getString("status")=="conflict")
        require(receipt.opt("changed")==false && (!receipt.has("committedRevision") || receipt.isNull("committedRevision")))
        val requestID=OwnedPhoneEdit.id(receipt.get("editID"))
        val profileID=OwnedPhoneEdit.id(receipt.get("profileID"))
        val base=OwnedPhoneEdit.revision(receipt.get("baseConfigurationRevision"))
        val index=values.indexOfFirst { it.serverID==serverID && it.edit.editID==requestID && it.edit.profileID==profileID && it.edit.baseConfigurationRevision==base }
        if(index<0) return values
        require(authority.serverID==serverID)
        val revision=requireNotNull(authority.configurationRevision)
        require(revision>=OwnedPhoneEdit.revision(receipt.get("configurationRevision")) && revision>base) { "Wait for an advancing authority snapshot" }
        require(newEditID!=requestID && values.none { it.serverID==serverID && it.edit.editID==newEditID })
        val pending=values[index]
        val retry=OwnedPhoneEdit.decode(StrictJSON.data(pending.edit.json().put("editID",newEditID.toString()).put("baseConfigurationRevision",revision)))
        // Prove all queued scalar intents still apply to the actual new profile.
        // A deleted profile/control stays pending for recovery instead of being recreated.
        var preview=authority.profiles.single { it.id==profileID }
        values.filter { it.serverID==serverID && it.edit.profileID==profileID }.forEach {
            preview=(if(it===pending) retry else it.edit).applying(preview,it.updatedAt)
        }
        val recovery=PendingOwnedPhoneEdit.Recovery(StrictJSON.data(preview.source).toString(Charsets.UTF_8),profileID)
        return values.mapIndexed { i,value ->
            if(i==index) PendingOwnedPhoneEdit.restored(serverID,retry,pending.updatedAt,recovery)
            else if(value.serverID==serverID && value.edit.profileID==profileID) value.replacingRecovery(recovery) else value
        }.also(::encode)
    }
    /** At most one in-flight edit per profile preserves the user's edit order. */
    fun uploads(values: List<PendingOwnedPhoneEdit>, serverID: String): List<PendingOwnedPhoneEdit> =
        values.filter { it.serverID==serverID }.distinctBy { it.edit.profileID }
}
