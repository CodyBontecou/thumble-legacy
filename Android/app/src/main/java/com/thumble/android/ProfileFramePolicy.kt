package com.thumble.android

import org.json.JSONArray
import org.json.JSONObject

/** Independent of real-time input v3; v5 remains unadvertised until full migration support ships. */
internal object ProfileFramePolicy {
    val supportedFormats = listOf(4)
    const val updateMessage = "Update Thumble on both devices to use this keypad format. Your saved setups and edits have been kept."
    fun advertise(message: JSONObject, formats: List<Int> = supportedFormats): JSONObject = message.put("supportedProfileFormats", JSONArray(formats))
    fun negotiate(required: Int, offered: List<Int>?): Int {
        val versions = offered ?: listOf(4)
        require(versions.size <= 8 && versions.toSet().size == versions.size && versions.all { it in 1..65535 }) { updateMessage }
        require(required in listOf(4, 5) && required in versions) { updateMessage }
        return required
    }
    fun validateStoredProfile(profile: JSONObject) {
        require(version(profile, "profileFormatVersion", 4) == 4) { updateMessage }
    }
    fun validateIncoming(message: JSONObject, formats: List<Int> = supportedFormats) {
        if (!message.has("gamepadProfiles") && !message.has("gamepadCustomization")) return
        var required = version(message, "profileFormatVersion", 4)
        require(required in listOf(4,5)) { updateMessage }
        message.optJSONArray("gamepadProfiles")?.let { profiles ->
            require(profiles.length() <= 256) { updateMessage }
            repeat(profiles.length()) { index ->
                val profile = profiles.getJSONObject(index)
                val bank = profile.has("appearanceBank") && !profile.isNull("appearanceBank")
                val resources = profile.has("appearanceResources") || profile.has("appearanceResourceVersion")
                val declared = version(profile, "profileFormatVersion", 4)
                require(declared in listOf(4,5)) { updateMessage }
                required = maxOf(required, declared, if (bank || resources) 5 else 4)
            }
        }
        negotiate(required, formats)
        if(required==5) {
            require(version(message,"profileFormatVersion",4)==5) { updateMessage }
            OwnedPhoneEdit.revision(message.opt("configurationRevision"))
            // Owned snapshots are complete canonical records. A legacy scalar
            // cannot replace the layout while leaving its bank/resources stale.
            val profiles=message.optJSONArray("gamepadProfiles") ?: error(updateMessage)
            repeat(profiles.length()) { require(version(profiles.getJSONObject(it),"profileFormatVersion",4)==5) { updateMessage } }
        }
    }
    private fun version(value: JSONObject, key: String, fallback: Int): Int {
        if (!value.has(key) || value.isNull(key)) return fallback
        val number = value.get(key) as? Number ?: error(updateMessage)
        val raw = number.toDouble()
        require(raw.isFinite() && raw == raw.toInt().toDouble()) { updateMessage }
        return raw.toInt()
    }
}
