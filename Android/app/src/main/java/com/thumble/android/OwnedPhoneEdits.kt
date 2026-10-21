package com.thumble.android

import org.json.JSONArray
import org.json.JSONObject
import java.math.BigInteger
import java.util.UUID

/** Immutable, bounded scalar intent. Never serializes a whole painted layout. */
internal class OwnedPhoneEdit private constructor(private val packet: String,
    val editID: UUID, val profileID: UUID, val baseConfigurationRevision: BigInteger) {
    fun json(): JSONObject = StrictJSON.objectValue(packet.toByteArray(), exactNumbers=true)
    fun data(): ByteArray = packet.toByteArray(Charsets.UTF_8)
    companion object {
        const val MAX_BYTES = 64 * 1024
        val MAX_REVISION: BigInteger = BigInteger("18446744073709551615")
        private val rootFields = setOf("schemaVersion","editID","profileID","baseConfigurationRevision","controlEdits","removedControlIDs","colorScheme")
        private val scalars = linkedMapOf("centerX" to .5,"centerY" to .5,"widthScale" to 1.0,"heightScale" to 1.0,"rotationDegrees" to 0.0)
        private val flags = setOf("isHidden","isLocationLocked")
        fun id(value: Any?): UUID {
            val raw=value as? String ?: error("Expected canonical UUID")
            return UUID.fromString(raw).also { require(it.toString().equals(raw,true) && it!=UUID(0,0)) }
        }
        fun revision(value: Any?): BigInteger {
            val text=(value as? Number)?.toString() ?: error("Expected integer revision")
            require(text.matches(Regex("[0-9]+")) && text.length<=20)
            return text.toBigInteger().also { require(it>BigInteger.ZERO && it<=MAX_REVISION) }
        }
        private fun integer(value: Any?, minimum: Int, maximum: Int): Int {
            val text=(value as? Number)?.toString() ?: error("Expected integer")
            require(text.matches(Regex("-?[0-9]+")) && text.length<=12)
            return text.toInt().also { require(it in minimum..maximum) }
        }
        fun decode(data: ByteArray): OwnedPhoneEdit {
            require(data.size<=MAX_BYTES)
            val raw=StrictJSON.objectValue(data,exactNumbers=true)
            require(raw.keys().asSequence().all { it in rootFields })
            require(integer(raw.opt("schemaVersion"),1,1)==1)
            val edit=id(raw.opt("editID"));val profile=id(raw.opt("profileID"));val revision=revision(raw.opt("baseConfigurationRevision"))
            val controls=if(raw.has("controlEdits")) raw.getJSONArray("controlEdits") else JSONArray()
            val removed=if(raw.has("removedControlIDs")) raw.getJSONArray("removedControlIDs") else JSONArray()
            require(controls.length()+removed.length()<=128)
            val owners=hashSetOf<UUID>()
            repeat(controls.length()) { index ->
                val control=controls.getJSONObject(index)
                require(control.keys().asSequence().all { it=="controlID" || it in scalars || it in flags || it=="zIndex" })
                require(owners.add(id(control.opt("controlID"))))
                var changes=0
                for((key,_) in scalars) if(control.has(key) && !control.isNull(key)) {
                    val value=(control.get(key) as? Number)?.toDouble() ?: error("Expected finite geometry")
                    val range=when(key) { "centerX","centerY"->0.0..1.0;"widthScale","heightScale"->.001..12.0;else->-36000.0..36000.0 }
                    require(value.isFinite() && value in range);changes++
                }
                for(key in flags) if(control.has(key) && !control.isNull(key)) { require(control.get(key) is Boolean);changes++ }
                if(control.has("zIndex") && !control.isNull("zIndex")) { integer(control.get("zIndex"),-100,100);changes++ }
                require(changes>0)
            }
            repeat(removed.length()) { require(owners.add(id(removed.get(it)))) }
            val color=raw.opt("colorScheme")?.takeUnless { it===JSONObject.NULL }
            require(color==null || color in setOf("system","light","dark"))
            require(owners.isNotEmpty() || color!=null)
            return OwnedPhoneEdit(data.toString(Charsets.UTF_8),edit,profile,revision)
        }
        private fun elements(layout: JSONObject): LinkedHashMap<UUID,JSONObject> {
            val result=linkedMapOf<UUID,JSONObject>();val array=layout.getJSONArray("elements")
            repeat(array.length()) { val element=array.getJSONObject(it);require(result.put(id(element.opt("id")),element)==null) }
            return result
        }
        private fun style(layout: JSONObject, element: JSONObject): JSONObject {
            val id=id(element.opt("id"))
            return if(element.optString("kind","button")=="button" && id.starterSlot()!=null)
                layout.swiftMap("buttonCustomizations")[id.toString().uppercase()] as? JSONObject ?: element.getJSONObject("layout")
            else element.getJSONObject("layout")
        }
        /** Derive exactly the geometry/hide/delete/appearance-mode changes the
         * phone editor supports. Paint, resources, bindings and canvas are kept. */
        fun between(profile: UUID, before: LayoutSnapshot, after: LayoutSnapshot, revision: BigInteger,
            editID: UUID=UUID.randomUUID()): OwnedPhoneEdit? {
            require(before.orientation==LayoutOrientation.LANDSCAPE && after.orientation==LayoutOrientation.LANDSCAPE)
            val a=before.json();val b=after.json();val old=elements(a);val next=elements(b)
            require(next.keys.all { it in old }) { "Adding controls requires an authority operation" }
            val changes=JSONArray();val removed=JSONArray()
            for((control,element) in old) {
                val newer=next[control]
                if(newer==null) { removed.put(control.toString());continue }
                val x=style(a,element);val y=style(b,newer);val patch=JSONObject().put("controlID",control.toString())
                for((key,fallback) in scalars) if(x.optDouble(key,fallback)!=y.optDouble(key,fallback)) patch.put(key,y.get(key))
                for(key in flags) if(x.optBoolean(key)!=y.optBoolean(key)) patch.put(key,y.get(key))
                if(x.optInt("zIndex")!=y.optInt("zIndex")) patch.put("zIndex",y.get("zIndex"))
                if(patch.length()>1) changes.put(patch)
            }
            val raw=JSONObject().put("schemaVersion",1).put("editID",editID.toString()).put("profileID",profile.toString())
                .put("baseConfigurationRevision",revision)
            if(changes.length()>0) raw.put("controlEdits",changes)
            if(removed.length()>0) raw.put("removedControlIDs",removed)
            if(a.optString("colorSchemePreference","system")!=b.optString("colorSchemePreference","system")) raw.put("colorScheme",b.getString("colorSchemePreference"))
            if(changes.length()==0 && removed.length()==0 && !raw.has("colorScheme")) {
                require(before.samePresentation(after)) { "Layout contains an unsupported edit" };return null
            }
            val result=decode(raw.toString().toByteArray())
            require(result.applying(before).samePresentation(after)) { "Layout contains an unsupported edit" }
            return result
        }
    }
    fun applying(snapshot: LayoutSnapshot): LayoutSnapshot {
        var current=snapshot
        val raw=json();val removed=raw.optJSONArray("removedControlIDs") ?: JSONArray()
        val declared=elements(snapshot.json()).keys
        repeat(removed.length()) { require(id(removed.get(it)) in declared) { "Removed control is missing" } }
        repeat(removed.length()) { current=LayoutControlPatch(id(removed.get(it)),delete=true).applying(current) }
        val layout=current.json();val controls=elements(layout)
        val changes=raw.optJSONArray("controlEdits") ?: JSONArray()
        repeat(changes.length()) { index ->
            val patch=changes.getJSONObject(index);val owner=id(patch.get("controlID"));val element=controls[owner] ?: error("Edited control is missing")
            val paint=StrictJSON.objectCopy(style(layout,element))
            patch.keys().forEach { key -> if(key!="controlID" && !patch.isNull(key)) paint.put(key,patch.get(key)) }
            element.put("layout",StrictJSON.objectCopy(paint))
            if(element.optString("kind","button")=="button" && owner.starterSlot()!=null) {
                val pairs=layout.swiftMap("buttonCustomizations").toMutableMap();pairs[owner.toString().uppercase()]=paint
                layout.put("buttonCustomizations",JSONArray().also { array -> pairs.forEach { (key,value) -> array.put(key).put(value) } })
            }
            layout.optJSONArray("customButtons")?.let { mirrors -> repeat(mirrors.length()) { i ->
                val mirror=mirrors.getJSONObject(i)
                if(id(mirror.get("id"))==owner) mirror.put("layout",StrictJSON.objectCopy(paint))
            } }
        }
        raw.optString("colorScheme").takeIf { it in setOf("system","light","dark") }?.let { layout.put("colorSchemePreference",it) }
        return LayoutSnapshot.capture(layout)
    }
    /** Local preview of a scalar intent. The authority remains responsible for
     * sidecar bindings and the committed configuration revision. */
    fun applying(profile: ControllerProfile, now: Long): ControllerProfile {
        require(profile.isOwned && profile.id==profileID && now>=0)
        val source=StrictJSON.objectCopy(profile.source)
        val layout=applying(LayoutSnapshot.capture(source.getJSONObject("customization"))).json()
        source.put("customization",layout)
        val removed=json().optJSONArray("removedControlIDs") ?: JSONArray()
        val originals=elements(profile.source.getJSONObject("customization"))
        val removedCustomOwners=(0 until removed.length()).map { id(removed.get(it)) }.filter { owner ->
            originals[owner]?.optString("kind","button")!="button" || owner.starterSlot()==null
        }
        source.optJSONObject("appearanceBank")?.getJSONArray("variants")?.let { variants ->
            repeat(variants.length()) { index ->
                val controls=variants.getJSONObject(index).getJSONObject("appearance").getJSONObject("controls")
                // Native deletion hides presets and retains their paint owner.
                removedCustomOwners.forEach { controls.remove(it.toString()) }
            }
        }
        source.put("updatedAt",maxOf(source.getLong("updatedAt"),now))
        return ControllerProfile.parse(source)
    }

}
