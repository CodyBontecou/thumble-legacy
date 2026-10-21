package com.thumble.android

import org.json.JSONArray
import org.json.JSONObject
import org.json.JSONTokener
import java.nio.ByteBuffer
import java.nio.charset.CodingErrorAction

/** Same duplicate-key and finite-number contract on Android and the JVM test runtime. */
internal class StrictJSONException(message: String): IllegalArgumentException(message)

internal object StrictJSON {
    fun objectValue(bytes: ByteArray, portable: Boolean = false, exactNumbers: Boolean = false): JSONObject {
        val text = Charsets.UTF_8.newDecoder().onMalformedInput(CodingErrorAction.REPORT)
            .onUnmappableCharacter(CodingErrorAction.REPORT).decode(ByteBuffer.wrap(bytes)).toString()
        return Reader(text,portable,exactNumbers).read() as? JSONObject ?: throw StrictJSONException("invalid-json")
    }
    /** org.json's writer can round an exact Number to a nearby integer.
     * Owned caches use this writer to preserve every numeric token. */
    fun data(value: JSONObject, maximumBytes: Int = WireCodec.MAX_PAYLOAD): ByteArray {
        val output=StringBuilder()
        fun write(node: Any?, depth: Int) {
            require(depth<=128)
            when(node) {
                null,JSONObject.NULL -> output.append("null")
                is JSONObject -> {
                    output.append('{')
                    node.keys().asSequence().forEachIndexed { index,key ->
                        if(index>0) output.append(',')
                        require(isWellFormed(key));output.append(JSONObject.quote(key)).append(':');write(node.get(key),depth+1)
                    }
                    output.append('}')
                }
                is JSONArray -> {
                    output.append('[')
                    repeat(node.length()) { index -> if(index>0) output.append(',');write(node.get(index),depth+1) }
                    output.append(']')
                }
                is String -> { require(isWellFormed(node));output.append(JSONObject.quote(node)) }
                is Boolean -> output.append(node)
                is Number -> { require(node.toDouble().isFinite());output.append(node.toString()) }
                else -> error("Unsupported JSON value")
            }
            require(output.length<=maximumBytes) { "JSON exceeds storage bound" }
        }
        write(value,0)
        return output.toString().toByteArray(Charsets.UTF_8).also { require(it.size<=maximumBytes) }
    }
    fun objectCopy(value: JSONObject): JSONObject = objectValue(data(value),exactNumbers=true)
    private fun canonicalKey(value: String): String = if (value.all { it.code < 128 }) value else
        java.text.Normalizer.normalize(value, java.text.Normalizer.Form.NFC)
    private fun isWellFormed(value: String): Boolean {
        var index = 0
        while (index < value.length) {
            val character = value[index++]
            if (character.isHighSurrogate()) {
                if (index == value.length || !value[index++].isLowSurrogate()) return false
            } else if (character.isLowSurrogate()) return false
        }
        return true
    }
    /** Keep revision integers and their JSON token kind exact for versioned
     * control messages. The default parser remains unchanged for old callers. */
    private class ExactNumber(private val token: String): Number() {
        override fun toByte() = toInt().toByte()
        override fun toShort() = toInt().toShort()
        override fun toInt() = token.toBigDecimal().toInt()
        override fun toLong() = token.toBigDecimal().toLong()
        override fun toFloat() = token.toFloat()
        override fun toDouble() = token.toDouble()
        override fun toString() = token
    }
    private class Reader(private val text: String,private val portable: Boolean,private val exactNumbers: Boolean) {
        private var at = 0
        private fun bad(): Nothing = throw StrictJSONException("invalid-json")
        private fun space() { while (at < text.length && text[at] in " \t\n\r") at++ }
        fun read(): Any { val result=value(0);space();if (at != text.length) bad();return result }
        private fun value(depth: Int): Any {
            if (depth > 128) bad()
            space();if (at == text.length) bad()
            return when (text[at]) {
                '{' -> {
                    if(portable && depth>=64) bad()
                    at++;space();val result=JSONObject();val seen=mutableSetOf<String>()
                    if (at < text.length && text[at] == '}') { at++;return result }
                    while (true) {
                        space();val key=string();if(portable && key.toByteArray(Charsets.UTF_8).size>256) bad()
                        if (!seen.add(canonicalKey(key))) throw StrictJSONException("duplicate-json-key")
                        space();if (at == text.length || text[at++] != ':') bad()
                        result.put(key,value(depth+1));if(portable && result.length()>4096) bad();space()
                        if (at == text.length) bad()
                        when (text[at++]) { '}' -> return result; ',' -> {}; else -> bad() }
                    }
                    @Suppress("UNREACHABLE_CODE") result
                }
                '[' -> {
                    if(portable && depth>=64) bad()
                    at++;space();val result=JSONArray()
                    if (at < text.length && text[at] == ']') { at++;return result }
                    while (true) {
                        result.put(value(depth+1));if(portable && result.length()>4096) bad();space();if (at == text.length) bad()
                        when (text[at++]) { ']' -> return result; ',' -> {}; else -> bad() }
                    }
                    @Suppress("UNREACHABLE_CODE") result
                }
                '"' -> string()
                't' -> literal("true",true)
                'f' -> literal("false",false)
                'n' -> literal("null",JSONObject.NULL)
                else -> {
                    val start=at
                    while (at < text.length && text[at] !in ",]} \t\n\r") at++
                    val number=text.substring(start,at)
                    if (!number.matches(Regex("-?(0|[1-9][0-9]*)(\\.[0-9]+)?([eE][+-]?[0-9]+)?"))) bad()
                    if(portable && number.none { it in ".eE" }) {
                        val magnitude=number.removePrefix("-")
                        if(magnitude.length>16 || magnitude.length==16 && magnitude>"9007199254740991") bad()
                    }
                    if (!number.toDouble().isFinite()) bad()
                    if (exactNumbers) ExactNumber(number) else number.toLongOrNull() ?: number.toDoubleOrNull()?.takeIf { it.isFinite() } ?: bad()
                }
            }
        }
        private fun literal(expected: String,result: Any): Any {
            if (!text.startsWith(expected,at)) bad();at+=expected.length;return result
        }
        private fun string(): String {
            if (at == text.length || text[at] != '"') bad()
            val start=at++
            while (at < text.length) {
                when (val c=text[at++]) {
                    '"' -> return (JSONTokener(text.substring(start,at)).nextValue() as? String)?.takeIf {
                        isWellFormed(it) && (!portable || it.toByteArray(Charsets.UTF_8).size<=256*1024)
                    } ?: bad()
                    '\\' -> {
                        if (at == text.length) bad()
                        when (text[at++]) {
                            '"','\\','/','b','f','n','r','t' -> {}
                            'u' -> { repeat(4) { if (at == text.length || text[at++].digitToIntOrNull(16) == null) bad() } }
                            else -> bad()
                        }
                    }
                    else -> if (c.code < 32) bad()
                }
            }
            bad()
        }
    }
}
